use super::*;

#[test]
fn test_serde_path() {
    let paths = [Path {
        date: String::from("a"),
        path: String::from("b"),
    }];
    let yaml = serde_yaml::to_string(&paths);
    assert!(yaml.is_ok());
    let new_paths_res: Result<Vec<Path>, serde_yaml::Error> =
        serde_yaml::from_str(yaml.unwrap().as_str());
    assert!(new_paths_res.is_ok());
    let new_paths = new_paths_res.unwrap();
    assert_eq!(new_paths.len(), 1);
    assert_eq!(new_paths[0].path, "b");
}

#[test]
fn test_serde_shortcut() {
    use crate::expimp::Shortcut;
    let shortcuts = [Shortcut {
        name: String::from("a"),
        path: String::from("b"),
        description: Some(String::from("c")),
    }];
    let yaml = serde_yaml::to_string(&shortcuts);
    assert!(yaml.is_ok());
    let new_shortcuts_res: Result<Vec<Shortcut>, serde_yaml::Error> =
        serde_yaml::from_str(yaml.unwrap().as_str());
    assert!(new_shortcuts_res.is_ok());
    let new_shortcuts = new_shortcuts_res.unwrap();
    assert_eq!(new_shortcuts.len(), 1);
    assert_eq!(new_shortcuts[0].name, "a");
    assert_eq!(new_shortcuts[0].path, "b");
    assert_eq!(new_shortcuts[0].description, Some(String::from("c")));
}

#[test]
fn test_load_shortcuts() {
    use crate::store::Store;
    let store = Store::setup_test_store();

    // Perform a simple load
    let shortcuts = vec![Shortcut {
        name: String::from("a"),
        path: String::from("b"),
        description: Some(String::from("c")),
    }];
    load_shortcuts(store.clone(), shortcuts);
    let rs = store.list_all_shortcuts();
    assert!(rs.is_ok());
    let list = rs.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].name, "a");
    assert_eq!(list[0].path, "b");
    assert_eq!(list[0].description, Some(String::from("c")));

    // Load again to test deletion of existing shortcut
    let shortcuts = vec![
        Shortcut {
            name: String::from("x"),
            path: String::from("y"),
            description: Some(String::from("z")),
        },
        Shortcut {
            name: String::from("a"),
            path: String::from("bb"),
            description: Some(String::from("cc")),
        },
    ];
    load_shortcuts(store.clone(), shortcuts);
    let rs = store.list_all_shortcuts();
    assert!(rs.is_ok());
    let list = rs.unwrap();
    assert_eq!(list.len(), 2);
    let shortcut_a = list.iter().find(|s| s.name == "a").unwrap();
    assert_eq!(shortcut_a.path, "bb");
    assert_eq!(shortcut_a.description, Some(String::from("cc")));

    let shortcut_x = list.iter().find(|s| s.name == "x").unwrap();
    assert_eq!(shortcut_x.path, "y");
    assert_eq!(shortcut_x.description, Some(String::from("z")));
}
