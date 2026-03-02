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

#[test]
fn test_export_import_shortcuts() {
    use tempfile::NamedTempFile;

    use crate::store::Store;
    let store = Store::setup_test_store();

    // Add some shortcuts
    store
        .add_shortcut("home", "/home/user", Some("Home directory"))
        .unwrap();
    store.add_shortcut("work", "/work/project", None).unwrap();

    // Export to a temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();
    export_shortcuts_to_yaml(store.clone(), temp_path.clone());

    // Create a new store and import
    let new_store = Store::setup_test_store();
    load_shortcuts_from_yaml(new_store.clone(), temp_path);

    // Verify the shortcuts were imported correctly
    let shortcuts = new_store.list_all_shortcuts().unwrap();
    assert_eq!(shortcuts.len(), 2);

    let home = shortcuts.iter().find(|s| s.name == "home").unwrap();
    assert_eq!(home.path, "/home/user");
    assert_eq!(home.description, Some(String::from("Home directory")));

    let work = shortcuts.iter().find(|s| s.name == "work").unwrap();
    assert_eq!(work.path, "/work/project");
    assert_eq!(work.description, None);
}

#[test]
fn test_export_import_current_paths() {
    use tempfile::NamedTempFile;

    use crate::store::Store;
    let store = Store::setup_test_store();

    // Add some paths
    store.add_path_with_time("/path/one", 1000).unwrap();
    store.add_path_with_time("/path/two", 2000).unwrap();

    // Export to a temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();
    export_paths_to_yaml(store.clone(), temp_path.clone());

    // Create a new store and import
    let new_store = Store::setup_test_store();
    load_paths_from_yaml(new_store.clone(), temp_path);

    // Verify the paths were imported correctly
    let paths = new_store.list_all_paths().unwrap();
    assert_eq!(paths.len(), 2);

    let path_one = paths.iter().find(|p| p.path == "/path/one").unwrap();
    assert_eq!(path_one.date, 1000);

    let path_two = paths.iter().find(|p| p.path == "/path/two").unwrap();
    assert_eq!(path_two.date, 2000);
}

#[test]
fn test_export_path_history() {
    use tempfile::NamedTempFile;

    use crate::store::Store;
    let store = Store::setup_test_store();

    // Add paths multiple times to create history
    store.add_path_with_time("/path/one", 1000).unwrap();
    store.add_path_with_time("/path/one", 1100).unwrap();
    store.add_path_with_time("/path/two", 2000).unwrap();

    // Export history to a temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();
    export_path_history_to_yaml(store.clone(), temp_path.clone());

    // Read and verify the exported file has all history entries
    let yaml_content = std::fs::read_to_string(&temp_path).unwrap();
    let exported_paths: Vec<Path> = serde_yaml::from_str(&yaml_content).unwrap();

    // Should have 3 entries in history (even though current paths has only 2)
    assert_eq!(exported_paths.len(), 3);

    // Verify we have both timestamps for /path/one
    let path_one_entries: Vec<_> = exported_paths
        .iter()
        .filter(|p| p.path == "/path/one")
        .collect();
    assert_eq!(path_one_entries.len(), 2);
}

#[test]
fn test_export_import_path_history() {
    use tempfile::NamedTempFile;

    use crate::store::Store;
    let store = Store::setup_test_store();

    // Add paths multiple times to create history
    store.add_path_with_time("/history/path/one", 1000).unwrap();
    store.add_path_with_time("/history/path/two", 2000).unwrap();
    store.add_path_with_time("/history/path/one", 1500).unwrap(); // Same path, different time
    store
        .add_path_with_time("/history/path/three", 3000)
        .unwrap();

    // Export history to a temporary file
    let temp_file = NamedTempFile::new().unwrap();
    let temp_path = temp_file.path().to_path_buf();
    export_path_history_to_yaml(store.clone(), temp_path.clone());

    // Verify the exported file
    let yaml_content = std::fs::read_to_string(&temp_path).unwrap();
    let exported_paths: Vec<Path> = serde_yaml::from_str(&yaml_content).unwrap();

    // Should have 4 entries in history
    assert_eq!(exported_paths.len(), 4);

    // Create a new store and import the history
    let new_store = Store::setup_test_store();
    import_path_history_from_yaml(new_store.clone(), temp_path);

    // Verify the history was imported correctly
    let imported_history = new_store.list_all_path_history().unwrap();
    assert_eq!(imported_history.len(), 4);

    // Verify we have both timestamps for /history/path/one
    let path_one_entries: Vec<_> = imported_history
        .iter()
        .filter(|p| p.path == "/history/path/one")
        .collect();
    assert_eq!(path_one_entries.len(), 2);

    // Verify the timestamps are correct
    let timestamps: Vec<i64> = path_one_entries.iter().map(|p| p.date).collect();
    assert!(timestamps.contains(&1000));
    assert!(timestamps.contains(&1500));

    // Verify other paths
    let path_two = imported_history
        .iter()
        .find(|p| p.path == "/history/path/two")
        .unwrap();
    assert_eq!(path_two.date, 2000);

    let path_three = imported_history
        .iter()
        .find(|p| p.path == "/history/path/three")
        .unwrap();
    assert_eq!(path_three.date, 3000);

    // Important: Verify that importing history does NOT update current paths
    let current_paths = new_store.list_all_paths().unwrap();
    // Current paths should be empty since we only imported to history
    assert_eq!(current_paths.len(), 0);
}
