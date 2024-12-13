use crate::config::Config;
use crate::help;
use crate::store;
use crate::store::{Path, Shortcut, Store};
use crate::tableview::{GuiResult, RowifyFn, TableView};
use std::cell::RefCell;

use log::debug;
use ratatui::text::{Line, Span};
use ratatui::{
    style::{Color, Stylize},
    widgets::Row,
    DefaultTerminal,
};

use std::env;
use std::rc::Rc;

/// The several views of the application
enum View {
    History,
    Shortcuts,
}

/// The main application structure
struct Gui<'a> {
    terminal: DefaultTerminal,
    current_view: View,
    history_view: TableView<'a, store::Path, bool>,
    shortcut_view: TableView<'a, store::Shortcut, bool>,
}

impl<'a> Gui<'a> {
    /// Return a Line with where HOME is replaced by '~'
    fn reduce_path(path: &Path) -> Line<'_> {
        let home = env::var("HOME");
        match home {
            Ok(home) => {
                let spm = home.clone() + "/";
                if path.path.starts_with(&(spm)) || path.path == home {
                    Span::from("~").fg(Color::DarkGray) + Span::from(&path.path[(spm.len() - 1)..])
                } else {
                    Line::from(Span::from(path.path.clone()))
                }
            }
            Err(_) => Line::from(Span::from(path.path.clone())),
        }
    }

    /// Return a function that formats a row for the history view
    fn build_format_history_row_builder(
        store: &'a store::Store,
        config: &'a Config,
        view_state: Rc<RefCell<bool>>,
    ) -> RowifyFn<'a, store::Path> {
        let view_state = view_state.clone();
        Box::new(move |paths| {
            let shortcuts: Vec<Shortcut> = store.list_all_shortcuts().unwrap();
            let date_color = config.colors.date.parse::<Color>().unwrap();
            let path_color = config.colors.path.parse::<Color>().unwrap();
            paths
                .iter()
                .map(|path| {
                    let shortened_line = match *view_state.borrow() {
                        true => Self::shorten_path(config, &shortcuts, path),
                        false => None,
                    };
                    let line = shortened_line.unwrap_or_else(|| Self::reduce_path(path));
                    vec![
                        Line::from(Span::from((config.date_formater)(path.date)).fg(date_color)),
                        line.fg(path_color),
                    ]
                })
                .map(Row::new)
                .collect()
        })
    }

    /// Return a Line where the longest matching shortcut path is replaced by the shortcut name
    fn shorten_path(
        config: &Config,
        shortcuts: &Vec<Shortcut>,
        path: &Path,
    ) -> Option<Line<'static>> {
        let mut shortened_line: Option<Line> = None;
        let mut cpath = "";
        let scc = config.colors.shortcut_name.parse::<Color>().unwrap();
        for shortcut in shortcuts {
            let spm = format!("{}/", shortcut.path);
            if (path.path.starts_with(&spm) || path.path == shortcut.path)
                && shortcut.path.len() > cpath.len()
            {
                cpath = shortcut.path.as_str();
                shortened_line = Some(
                    Span::from("[").fg(scc)
                        + Span::from(shortcut.name.clone()).fg(scc)
                        + Span::from("]").fg(scc)
                        + Span::from(String::from(&path.path[(spm.len() - 1)..])),
                );
            }
        }
        shortened_line
    }

    /// Build the history view
    fn build_history_view(
        store: &'a Store,
        config: &'a Config,
        view_state: &Rc<RefCell<bool>>,
    ) -> TableView<'a, Path, bool> {
        TableView::new(
            vec!["date".to_string(), "path".to_string()],
            Box::new(|pos, len, text| store.list_paths(pos, len, text)),
            Box::new(Gui::build_format_history_row_builder(
                store,
                config,
                view_state.clone(),
            )),
            |path| path.path.clone(),
            config,
            view_state.clone(),
            Box::new(|path| {
                debug!("delete path: {}", path.path);
                store.delete_path_by_id(path.id).unwrap();
            }),
        )
    }

    /// Build the shortcut view
    fn build_shortcut_view(
        store: &'a Store,
        config: &'a Config,
        view_state: Rc<RefCell<bool>>,
    ) -> TableView<'a, Shortcut, bool> {
        TableView::new(
            vec!["shortcut".to_string(), "path".to_string()],
            Box::new(|pos: usize, len: usize, text: &str| store.list_shortcuts(pos, len, text)),
            Box::new(|shortcuts: &Vec<store::Shortcut>| {
                let scc = config.colors.shortcut_name.parse::<Color>().unwrap();
                let path_color = config.colors.path.parse::<Color>().unwrap();
                shortcuts
                    .iter()
                    .map(|shortcut| {
                        Row::new(vec![
                            Line::from(Span::from(shortcut.name.clone()).fg(scc)),
                            Line::from(Span::from(shortcut.path.clone())).fg(path_color),
                        ])
                    })
                    .collect()
            }),
            |shortcut: &store::Shortcut| shortcut.path.clone(),
            config,
            view_state.clone(),
            Box::new(|path| {
                debug!("delete shortcut: {}", path.path);
                store.delete_shortcut_by_id(path.id).unwrap();
            }),
        )
    }

    /// Instantiate the application GUI
    fn new(store: &'a store::Store, config: &'a Config) -> Gui<'a> {
        let view_state = Rc::<RefCell<bool>>::new(RefCell::new(true));
        Gui {
            terminal: ratatui::init(),
            current_view: View::History,
            history_view: Self::build_history_view(store, config, &view_state),
            shortcut_view: Self::build_shortcut_view(store, config, view_state),
        }
    }

    /// Run the application GUI loop
    fn run(&mut self) -> Option<String> {
        loop {
            let res = match self.current_view {
                View::History => self.history_view.run(&mut self.terminal),
                View::Shortcuts => self.shortcut_view.run(&mut self.terminal),
            };
            match res {
                GuiResult::Quit => {
                    ratatui::restore();
                    return None;
                }
                GuiResult::Print(str) => {
                    ratatui::restore();
                    return Some(str);
                }
                GuiResult::Next => match self.current_view {
                    View::History => self.current_view = View::Shortcuts,
                    View::Shortcuts => self.current_view = View::History,
                },
                GuiResult::Help => {
                    debug!("Help requested");
                    help::help_run(&mut self.terminal);
                }
            }
        }
    }
}

/// Launch the GUI. Returns the selected path or None if the user quit.
pub(crate) fn gui(store: store::Store, config: Config) -> Option<String> {
    color_eyre::install().unwrap();
    debug!("HistoryView::new()");
    let mut gui = Gui::new(&store, &config);
    gui.run()
}
