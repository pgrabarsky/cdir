use std::env;

use ratatui::style::Style;

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
        shortcut: None,
        smart_path: false,
    };
    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 80);
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
        shortcut: None,
        smart_path: false,
    };
    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 80);
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
    let path = Path::new(1, "/home/user/docs/work".to_string(), 0, &shortcuts);
    let result = Gui::shorten_path_for_path(&config, &path, 80);
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
        shortcut: None,
        smart_path: false,
    };
    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 80);
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
        shortcut: None,
        smart_path: false,
    };
    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 14);
    assert!(result.is_some());
    let line = result.unwrap();
    let line_str = line.to_string();
    assert_eq!(line_str, "[docs]/project");

    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 13);
    assert!(result.is_some());
    let line = result.unwrap();
    let line_str = line.to_string();
    assert_eq!(line_str, "[docs]/*oject");

    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 9);
    assert!(result.is_some());
    let line = result.unwrap();
    let line_str = line.to_string();
    assert_eq!(line_str, "[docs]/*t");

    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 8);
    assert!(result.is_some());
    let line = result.unwrap();
    let line_str = line.to_string();
    assert_eq!(line_str, "[docs]/*");

    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 7);
    assert!(result.is_some());
    let line = result.unwrap();
    let line_str = line.to_string();
    assert_eq!(line_str, "[docs]*");

    let result = Gui::shorten_path_for_shortcut(&config, &shortcuts, &path.path, 6);
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
        shortcut: None,
        smart_path: false,
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
        shortcut: None,
        smart_path: false,
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
        shortcut: None,
        smart_path: false,
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
        shortcut: None,
        smart_path: false,
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
        shortcut: None,
        smart_path: false,
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
        shortcut: None,
        smart_path: false,
    };

    let line = Gui::reduce_path(path.path.clone(), 19, Style::new());
    let line_str = line.to_string();
    assert_eq!(line_str, "/other/path/project");

    let line = Gui::reduce_path(path.path.clone(), 18, Style::new());
    let line_str = line.to_string();
    assert_eq!(line_str, "*ther/path/project");
}
