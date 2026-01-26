use std::{
    env,
    rc::Rc,
    sync::{Arc, Mutex},
};

use log::debug;
use ratatui::{
    layout::Constraint,
    style::Style,
    text::{Line, Span},
    widgets::Row,
};

use crate::{
    config::Config,
    help::Help,
    history_view_container::HistoryViewContainer,
    search_text_view::SearchTextState,
    shortcut_editor::ShortcutEditor,
    shortcut_view_container::ShortcutViewContainer,
    store::{self, Path, Shortcut, Store},
    tableview::{RowifyFn, TableViewState},
    tui::{ViewBuilder, ViewManager},
};

const HISTORY_VIEW_CONTAINER: u16 = 0;
const SHORTCUT_VIEW_ID: u16 = 1;

/// The main application structure
pub(crate) struct Gui {
    table_view_state: Arc<Mutex<TableViewState>>,
    history_view_container: Option<ViewBuilder>,
    shortcut_view_container: Option<ViewBuilder>,
}

impl Gui {
    /// Return a Line with where HOME is replaced by '~'
    pub(crate) fn reduce_path(path: String, size: u16, home_tild_style: Style) -> Line<'static> {
        if size == 0 {
            return Line::from("");
        }

        let home = env::var("HOME");
        match home {
            Ok(home) => {
                let spm = home.clone() + "/";
                if path.starts_with(&spm) || path == home.as_str() {
                    Self::do_reduce_path(&path, home, size, home_tild_style)
                } else {
                    Self::reduce_string(&path, size as usize)
                }
            }
            Err(_) => Self::reduce_string(&path, size as usize),
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
        store: store::Store,
        config: Arc<Config>,
        table_view_state: Arc<Mutex<TableViewState>>,
    ) -> RowifyFn<store::Path> {
        let table_view_state = table_view_state.clone();
        let store = store.clone();
        Box::new(move |paths: &[Path], size: &[u16]| {
            let shortcuts: Vec<Shortcut> = store.list_all_shortcuts().unwrap();
            let table_view_state = table_view_state.clone();
            let config = config.clone();
            paths
                .iter()
                .map(move |path| {
                    let path = path.clone();
                    // format the date
                    let date: Line = Line::from(
                        Span::from((config.date_formater)(path.date))
                            .style(config.styles.date_style),
                    );

                    // format the path
                    let shortened_line =
                        match table_view_state.lock().unwrap().display_with_shortcuts {
                            true => Self::shorten_path(
                                config.as_ref(),
                                &shortcuts,
                                &path.path,
                                size[1],
                                true,
                            ),
                            false => None,
                        };
                    let path = shortened_line
                        .unwrap_or_else(|| {
                            Self::reduce_path(path.path, size[1], config.styles.home_tilde_style)
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
        &mut self,
        view_manager: Rc<ViewManager>,
        store: Store,
        config: Arc<Config>,
        search_text_state: Arc<Mutex<SearchTextState>>,
    ) {
        self.history_view_container = Some(HistoryViewContainer::builder(
            view_manager.clone(),
            vec!["date".to_string(), "path".to_string()],
            vec![Constraint::Length(20), Constraint::Fill(1)],
            {
                let store = store.clone();
                Box::new(move |pos, len, text, fuzzy| store.list_paths(pos, len, text, fuzzy))
            },
            Box::new(Gui::build_format_history_row_builder(
                store.clone(),
                config.clone(),
                self.table_view_state.clone(),
            )),
            |path: &Path| path.path.clone(),
            config.clone(),
            self.table_view_state.clone(),
            {
                let store = store.clone();
                Box::new(move |path| {
                    debug!("delete path: {}", path.path);
                    store.delete_path_by_id(path.id).unwrap();
                })
            },
            //search_string,
            None,
            search_text_state,
        ));
    }

    /// Return a function that formats a row for the history view
    fn build_format_shortcut_row_builder(
        store: Store,
        config: Arc<Config>,
        table_view_state: Arc<Mutex<TableViewState>>,
    ) -> RowifyFn<store::Shortcut> {
        let table_view_state = table_view_state.clone();
        let store = store.clone();
        let config = config.clone();
        Box::new(move |shortcuts: &[Shortcut], size: &[u16]| {
            shortcuts
                .iter()
                .map(|shortcut| {
                    // format the path
                    let shortcut = shortcut.clone();
                    let shortened_line =
                        match table_view_state.lock().unwrap().display_with_shortcuts {
                            true => {
                                let all_shortcuts: Vec<Shortcut> =
                                    store.list_all_shortcuts().unwrap();
                                Self::shorten_path(
                                    config.as_ref(),
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
                                shortcut.path,
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
                        Line::from(
                            shortcut
                                .description
                                .clone()
                                .unwrap_or_else(|| "".to_string()),
                        )
                        .style(config.styles.description_style),
                    ])
                })
                .collect()
        })
    }

    /// Build the shortcut view
    fn build_shortcut_view(
        &mut self,
        view_manager: Rc<ViewManager>,
        store: Store,
        config: Arc<Config>,
        search_text_state: Arc<Mutex<SearchTextState>>,
    ) {
        let modal_store = store.clone();
        let modal_config = config.clone();
        let editor_modal_view_builder = Box::new(move |shortcut: Shortcut| {
            Box::new(ShortcutEditor::builder(
                modal_store.clone(),
                modal_config.clone(),
                shortcut.clone(),
            ))
        });

        self.shortcut_view_container = Some(ShortcutViewContainer::builder(
            view_manager.clone(),
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
            {
                let store = store.clone();
                Box::new(move |pos, len, text, fuzzy| store.list_shortcuts(pos, len, text, fuzzy))
            },
            Box::new(Gui::build_format_shortcut_row_builder(
                store.clone(),
                config.clone(),
                self.table_view_state.clone(),
            )),
            |shortcut: &store::Shortcut| shortcut.path.clone(),
            config.clone(),
            self.table_view_state.clone(),
            {
                let store = store.clone();
                Box::new(move |path| {
                    debug!("delete shortcut: {}", path.path);
                    store.delete_shortcut_by_id(path.id).unwrap();
                })
            },
            //search_string,
            Some(editor_modal_view_builder),
            search_text_state,
        ));
    }

    /// Instantiate the application GUI
    fn new(view_manager: Rc<ViewManager>, store: store::Store, config: Arc<Config>) -> Gui {
        let mut gui = Gui {
            table_view_state: Arc::new(Mutex::new(TableViewState::new())),
            history_view_container: None,
            shortcut_view_container: None,
        };
        let search_text_state = Arc::new(Mutex::new(SearchTextState::new(view_manager.clone())));
        gui.build_history_view(
            view_manager.clone(),
            store.clone(),
            config.clone(),
            search_text_state.clone(),
        );
        gui.build_shortcut_view(
            view_manager.clone(),
            store.clone(),
            config.clone(),
            search_text_state.clone(),
        );

        gui
    }

    /// Run the application GUI loop
    async fn run(&mut self, view_manager: Rc<ViewManager>) -> Option<String> {
        let vb = self.history_view_container.take().unwrap();
        view_manager.add_view(
            HISTORY_VIEW_CONTAINER,
            vb,
            &[HISTORY_VIEW_CONTAINER as usize],
        );

        let vb = self.shortcut_view_container.take().unwrap();
        view_manager.add_view(SHORTCUT_VIEW_ID, vb, &[SHORTCUT_VIEW_ID as usize]);

        view_manager.event_loop().await
    }
}

/// Launch the GUI. Returns the selected path or None if the user quit.
pub(crate) async fn gui(store: store::Store, config: Arc<Config>) -> Option<String> {
    debug!("gui");
    let mut view_manager: Rc<ViewManager> = Rc::new(ViewManager::new());

    if let Some(vm) = Rc::get_mut(&mut view_manager) {
        let config = config.clone();
        vm.set_global_help_view(Box::new(move || Help::builder(config.styles.clone())))
    }

    let mut gui = Gui::new(view_manager.clone(), store, config);
    gui.run(view_manager).await
}

#[cfg(test)]
mod tests {
    use std::env;

    use super::*;
    use crate::{
        config::Config,
        store::{Path, Shortcut},
    };

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
        let line = Gui::reduce_path(path.path, 80, Style::new());
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
        let line = Gui::reduce_path(path.path, 80, Style::new());
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
        let line = Gui::reduce_path(path.path, 80, Style::new());
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

        let line = Gui::reduce_path(path.path.clone(), 9, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/project");

        let line = Gui::reduce_path(path.path.clone(), 8, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/*oject");

        let line = Gui::reduce_path(path.path.clone(), 4, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/*t");

        let line = Gui::reduce_path(path.path.clone(), 3, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~/*");

        let line = Gui::reduce_path(path.path.clone(), 2, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~*");

        let line = Gui::reduce_path(path.path.clone(), 1, Style::new());
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

        let line = Gui::reduce_path(path.path.clone(), 2, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "~");

        let line = Gui::reduce_path(path.path.clone(), 1, Style::new());
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

        let line = Gui::reduce_path(path.path.clone(), 19, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "/other/path/project");

        let line = Gui::reduce_path(path.path.clone(), 18, Style::new());
        let line_str = line.to_string();
        assert_eq!(line_str, "*ther/path/project");
    }
}
