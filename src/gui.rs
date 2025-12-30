use crate::config::Config;
use crate::help;
use crate::store;
use crate::store::{Path, Shortcut, Store};
use crate::tableview::{GuiResult, RowifyFn, TableView};
use std::cell::RefCell;

use log::debug;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::{DefaultTerminal, widgets::Row};

use crate::shortcut_editor::ShortcutEditor;
use ratatui::layout::Constraint;
use std::env;
use std::rc::Rc;
use std::sync::{Arc, Mutex};

/// The several views of the application
enum View {
    History,
    Shortcuts,
}

/// The main application structure
pub(crate) struct Gui<'a> {
    config: &'a Config,
    terminal: DefaultTerminal,
    current_view: View,
    history_view: TableView<'a, store::Path, bool>,
    shortcut_view: TableView<'a, store::Shortcut, bool>,
}

impl<'a> Gui<'a> {
    /// Return a Line with where HOME is replaced by '~'
    pub(crate) fn reduce_path(path: &String, size: u16, home_tild_style: Style) -> Line<'_> {
        if size == 0 {
            return Line::from("");
        }

        let home = env::var("HOME");
        match home {
            Ok(home) => {
                let spm = home.clone() + "/";
                if path.starts_with(&spm) || path == home.as_str() {
                    Self::do_reduce_path(path, home, size, home_tild_style)
                } else {
                    Self::reduce_string(path, size as usize)
                }
            }
            Err(_) => Self::reduce_string(path, size as usize),
        }
    }

    fn reduce_string(path: &str, size: usize) -> Line<'static> {
        if path.len() <= size {
            return Line::from(Span::from(path.to_string()));
        }
        let remaining_size = size - 1;
        let path_suffix = &path[path.len() - remaining_size..];
        Line::from(Span::from(format!("*{}", path_suffix)))
    }

    fn do_reduce_path(
        path: &String,
        home: String,
        size: u16,
        home_tild_style: Style,
    ) -> Line<'static> {
        if path == &home {
            return Line::from(Span::from("~").style(home_tild_style));
        }

        if size == 1 {
            return Line::from("*");
        } else if size == 2 {
            return Span::from("~").style(home_tild_style) + Span::from("*");
        } else if size == 3 {
            return Span::from("~").style(home_tild_style) + Span::from("/*");
        }

        let path_suffix = &path[home.len() + 1..];
        let remaining_size = size as usize - 2; // for '~' and '/'
        if path_suffix.len() > remaining_size {
            let start_index = path_suffix.len() - remaining_size + 1;
            let path_suffix = format!("*{}", &path_suffix[start_index..]);
            return Span::from("~").style(home_tild_style)
                + Span::from("/")
                + Span::from(path_suffix);
        }

        Span::from("~").style(home_tild_style) + Span::from(path[home.len()..].to_string())
    }

    /// Return a Line where the longest matching shortcut path is replaced by the shortcut name
    /// If no substitution is possible, return None
    pub(crate) fn shorten_path(
        config: &Config,
        shortcuts: &[Shortcut],
        path: &String,
        size: u16,
        allow_shortcut_exact_match: bool,
    ) -> Option<Line<'static>> {
        if size == 0 {
            return None;
        }

        let mut shortened_line: Option<Line> = None;
        let mut cpath = "";
        for shortcut in shortcuts {
            if !allow_shortcut_exact_match && path == &shortcut.path {
                continue;
            }
            let spm = format!("{}/", shortcut.path);
            if (path.starts_with(&spm) || path == shortcut.path.as_str())
                && shortcut.path.len() > cpath.len()
            {
                cpath = shortcut.path.as_str();
                shortened_line = Some(Self::do_shorten_path(
                    path,
                    &config.styles.shortcut_name_style,
                    shortcut,
                    size,
                ));
            }
        }
        shortened_line
    }

    fn do_shorten_path(
        path: &String,
        style: &Style,
        shortcut: &Shortcut,
        size: u16,
    ) -> Line<'static> {
        if shortcut.name.len() + 3 == size as usize {
            return Span::from("[").style(*style)
                + Span::from(shortcut.name.clone()).style(*style)
                + Span::from("]").style(*style)
                + Span::from("*");
        } else if shortcut.name.len() + 3 > size as usize {
            return Line::from("*");
        }
        let mut result_path = Span::from("[").style(*style)
            + Span::from(shortcut.name.clone()).style(*style)
            + Span::from("]").style(*style);

        // if the path is an exact match of the shortcut, return it directly
        if path == shortcut.path.as_str() {
            return result_path;
        }

        // else we need to adjust the text if it's too long...

        // We want to keep the / after the shortcut name
        result_path += Span::from("/");

        let remaining_size = size as usize - (shortcut.name.len() + 3);

        // take the suffix of the path after the shortcut path and after '/'
        let path_suffix = &path[shortcut.path.len() + 1..];

        if path_suffix.len() > remaining_size {
            let start_index = path_suffix.len() - remaining_size + 1;
            let path_suffix = format!("*{}", &path_suffix[start_index..]);
            result_path += Span::from(path_suffix);
            return result_path;
        }

        result_path += Span::from(String::from(path_suffix));

        result_path
    }

    /// Return a function that formats a row for the history view
    fn build_format_history_row_builder(
        store: &'a store::Store,
        config: &'a Config,
        view_state: Rc<RefCell<bool>>,
    ) -> RowifyFn<'a, store::Path> {
        let view_state = view_state.clone();
        Box::new(move |paths: &[Path], size: &[u16]| {
            let shortcuts: Vec<Shortcut> = store.list_all_shortcuts().unwrap();
            paths
                .iter()
                .map(|path| {
                    // format the date
                    let date: Line = Line::from(
                        Span::from((config.date_formater)(path.date))
                            .style(config.styles.date_style),
                    );

                    // format the path
                    let shortened_line = match *view_state.borrow() {
                        true => Self::shorten_path(config, &shortcuts, &path.path, size[1], true),
                        false => None,
                    };
                    let path = shortened_line
                        .unwrap_or_else(|| {
                            Self::reduce_path(&path.path, size[1], config.styles.home_tilde_style)
                        })
                        .style(config.styles.path_style);

                    vec![date, path]
                })
                .map(Row::new)
                .collect()
        })
    }

    /// Build the history view
    fn build_history_view(
        store: &'a Store,
        config: &'a Config,
        view_state: &Rc<RefCell<bool>>,
        search_string: Arc<Mutex<String>>,
    ) -> TableView<'a, Path, bool> {
        TableView::new(
            vec!["date".to_string(), "path".to_string()],
            vec![Constraint::Length(20), Constraint::Fill(1)],
            Box::new(|pos, len, text, fuzzy| store.list_paths(pos, len, text, fuzzy)),
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
            search_string,
            None,
        )
    }

    /// Return a function that formats a row for the history view
    fn build_format_shortcut_row_builder(
        store: &'a store::Store,
        config: &'a Config,
        view_state: Rc<RefCell<bool>>,
    ) -> RowifyFn<'a, store::Shortcut> {
        let view_state = view_state.clone();
        Box::new(move |shortcuts: &[Shortcut], size: &[u16]| {
            shortcuts
                .iter()
                .map(|shortcut| {
                    // format the path
                    let shortened_line = match *view_state.borrow() {
                        true => {
                            let all_shortcuts: Vec<Shortcut> = store.list_all_shortcuts().unwrap();
                            Self::shorten_path(
                                config,
                                &all_shortcuts,
                                &shortcut.path,
                                size[1],
                                false,
                            )
                        }
                        false => None,
                    };
                    let path = shortened_line
                        .unwrap_or_else(|| {
                            Self::reduce_path(
                                &shortcut.path,
                                size[1],
                                config.styles.home_tilde_style,
                            )
                        })
                        .style(config.styles.path_style);

                    Row::new(vec![
                        Line::from(
                            Span::from(shortcut.name.clone())
                                .style(config.styles.shortcut_name_style),
                        ),
                        path,
                        Line::from(shortcut.description.as_ref().map_or("", |s| s.as_str()))
                            .style(config.styles.description_style),
                    ])
                })
                .collect()
        })
    }

    /// Build the shortcut view
    fn build_shortcut_view(
        store: &'a Store,
        config: &'a Config,
        view_state: Rc<RefCell<bool>>,
        search_string: Arc<Mutex<String>>,
    ) -> TableView<'a, Shortcut, bool> {
        TableView::new(
            vec![
                "shortcut".to_string(),
                "path".to_string(),
                "description".to_string(),
            ],
            vec![
                Constraint::Length(20),
                Constraint::Fill(1),
                Constraint::Fill(1),
            ],
            Box::new(|pos: usize, len: usize, text: &str, fuzzy| {
                store.list_shortcuts(pos, len, text, fuzzy)
            }),
            Box::new(Gui::build_format_shortcut_row_builder(
                store,
                config,
                view_state.clone(),
            )),
            |shortcut: &store::Shortcut| shortcut.path.clone(),
            config,
            view_state.clone(),
            Box::new(|path| {
                debug!("delete shortcut: {}", path.path);
                store.delete_shortcut_by_id(path.id).unwrap();
            }),
            search_string,
            Some(Box::new(ShortcutEditor::new(store.clone(), config.clone()))),
        )
    }

    /// Instantiate the application GUI
    fn new(store: &'a store::Store, config: &'a Config) -> Gui<'a> {
        let view_state = Rc::<RefCell<bool>>::new(RefCell::new(true));
        let search_string = Arc::new(Mutex::new(String::new()));
        Gui {
            config,
            terminal: ratatui::init(),
            current_view: View::History,
            history_view: Self::build_history_view(
                store,
                config,
                &view_state,
                search_string.clone(),
            ),
            shortcut_view: Self::build_shortcut_view(
                store,
                config,
                view_state,
                search_string.clone(),
            ),
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
                    help::help_run(&mut self.terminal, self.config);
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;
    use crate::store::Path;
    use crate::store::Shortcut;
    use std::env;

    #[test]
    fn test_shorten_path_basic() {
        let config = Config::default();
        let shortcuts = vec![Shortcut {
            id: 1,
            name: "docs".to_string(),
            path: "/home/user/docs".to_string(),
            description: None,
        }];
        let path = Path {
            id: 1,
            path: "/home/user/docs/project".to_string(),
            date: 0,
        };
        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 80, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[docs]/project");
    }

    #[test]
    fn test_shorten_path_no_match() {
        let config = Config::default();
        let shortcuts = vec![Shortcut {
            id: 1,
            name: "docs".to_string(),
            path: "/home/user/docs".to_string(),
            description: None,
        }];
        let path = Path {
            id: 1,
            path: "/home/user/other/project".to_string(),
            date: 0,
        };
        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 80, true);
        assert!(result.is_none());
    }

    #[test]
    fn test_shorten_path_exact_match() {
        let config = Config::default();
        let shortcuts = vec![
            Shortcut {
                id: 1,
                name: "docs".to_string(),
                path: "/home/user/docs".to_string(),
                description: None,
            },
            Shortcut {
                id: 2,
                name: "work".to_string(),
                path: "/home/user/docs/work".to_string(),
                description: None,
            },
        ];
        let path = Path {
            id: 1,
            path: "/home/user/docs/work".to_string(),
            date: 0,
        };
        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 80, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[work]");
    }

    #[test]
    fn test_shorten_path_longest_match() {
        let config = Config::default();
        let shortcuts = vec![
            Shortcut {
                id: 1,
                name: "docs".to_string(),
                path: "/home/user/docs".to_string(),
                description: None,
            },
            Shortcut {
                id: 2,
                name: "work".to_string(),
                path: "/home/user/docs/work".to_string(),
                description: None,
            },
        ];
        let path = Path {
            id: 1,
            path: "/home/user/docs/work/project".to_string(),
            date: 0,
        };
        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 80, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[work]/project");
    }

    #[test]
    fn test_shorten_path_limited_size() {
        let config = Config::default();
        let shortcuts = vec![Shortcut {
            id: 1,
            name: "docs".to_string(),
            path: "/home/user/docs".to_string(),
            description: None,
        }];
        let path = Path {
            id: 1,
            path: "/home/user/docs/project".to_string(),
            date: 0,
        };
        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 14, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[docs]/project");

        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 13, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[docs]/*oject");

        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 9, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[docs]/*t");

        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 8, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[docs]/*");

        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 7, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "[docs]*");

        let result = Gui::shorten_path(&config, &shortcuts, &path.path, 6, true);
        assert!(result.is_some());
        let line = result.unwrap();
        let line_str = line.to_string();
        assert_eq!(line_str, "*");
    }

    #[test]
    fn test_reduce_path_home_replacement() {
        // Set HOME to a known value
        let home = "/home/testuser";
        unsafe {
            env::set_var("HOME", home);
        }
        let path = Path {
            id: 1,
            path: format!("{}/project", home),
            date: 0,
        };
        let line = Gui::reduce_path(&path.path, 80, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/project");
    }

    #[test]
    fn test_reduce_path_exact_home() {
        let home = "/home/testuser";
        unsafe {
            env::set_var("HOME", home);
        }
        let path = Path {
            id: 1,
            path: home.to_string(),
            date: 0,
        };
        let line = Gui::reduce_path(&path.path, 80, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~");
    }

    #[test]
    fn test_reduce_path_no_home_match() {
        unsafe {
            env::set_var("HOME", "/home/testuser");
        }
        let path = Path {
            id: 1,
            path: "/other/path/project".to_string(),
            date: 0,
        };
        let line = Gui::reduce_path(&path.path, 80, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "/other/path/project");
    }

    #[test]
    fn test_reduce_path_with_home_limited_size() {
        let home = "/home/testuser";
        unsafe {
            env::set_var("HOME", home);
        }
        let path = Path {
            id: 1,
            path: format!("{}/project", home),
            date: 0,
        };

        let line = Gui::reduce_path(&path.path, 9, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/project");

        let line = Gui::reduce_path(&path.path, 8, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/*oject");

        let line = Gui::reduce_path(&path.path, 4, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/*t");

        let line = Gui::reduce_path(&path.path, 3, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/*");

        let line = Gui::reduce_path(&path.path, 2, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~*");

        let line = Gui::reduce_path(&path.path, 1, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "*");
    }

    #[test]
    fn test_reduce_path_with_home_exact_limited_size() {
        let home = "/home/testuser";
        unsafe {
            env::set_var("HOME", home);
        }
        let path = Path {
            id: 1,
            path: home.to_string(),
            date: 0,
        };

        let line = Gui::reduce_path(&path.path, 2, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~");

        let line = Gui::reduce_path(&path.path, 1, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~");
    }

    #[test]
    fn test_reduce_path_without_home_limited_size() {
        let home = "/home/testuser";
        unsafe {
            env::set_var("HOME", home);
        }
        let path = Path {
            id: 1,
            path: "/other/path/project".to_string(),
            date: 0,
        };

        let line = Gui::reduce_path(&path.path, 19, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "/other/path/project");

        let line = Gui::reduce_path(&path.path, 18, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "*ther/path/project");
    }
}
