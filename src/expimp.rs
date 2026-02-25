use std::{fs, path::PathBuf};

use log::{error, info};
use serde::{Deserialize, Serialize};

use crate::store::Store;

#[cfg(test)]
#[path = "expimp_tests.rs"]
mod expimp_tests;

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Path {
    date: String,
    path: String,
}

/// Load paths from a YAML file and add them to the store.
/// The YAML file should contain a list of objects with `date` and `path` fields.
/// The `date` field should be a string representing a UNIX timestamp in seconds.
pub(crate) fn load_paths_from_yaml(store: Store, yaml_file: PathBuf) {
    if !yaml_file.exists() {
        error!("File {} does not exist", yaml_file.display());
        return;
    }
    match fs::read_to_string(&yaml_file) {
        Ok(contents) => {
            let new_paths_res: Result<Vec<Path>, serde_yaml::Error> =
                serde_yaml::from_str(contents.as_str());
            match new_paths_res {
                Ok(new_paths) => {
                    load_paths(store, new_paths);
                }
                Err(e) => {
                    error!("Failed to parse the file {}: {}", yaml_file.display(), e);
                }
            }
        }
        Err(e) => {
            error!("Failed to read file {}: {}", yaml_file.display(), e);
        }
    }
}

fn load_paths(store: Store, new_paths: Vec<Path>) {
    for entry in new_paths {
        match entry.date.parse::<u64>() {
            Ok(sec) => {
                let _ = store
                    .add_path_with_time(&entry.path, sec)
                    .map_err(|e| error!("{}", e));
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }
}

/// Load path history from a YAML file and add them to the store's paths_history table.
/// The YAML file should contain a list of objects with `date` and `path` fields.
/// The `date` field should be a string representing a UNIX timestamp in seconds.
/// Unlike load_paths_from_yaml, this function adds directly to paths_history without updating current paths.
pub(crate) fn import_path_history_from_yaml(store: Store, yaml_file: PathBuf) {
    if !yaml_file.exists() {
        error!("File {} does not exist", yaml_file.display());
        return;
    }
    match fs::read_to_string(&yaml_file) {
        Ok(contents) => {
            let new_paths_res: Result<Vec<Path>, serde_yaml::Error> =
                serde_yaml::from_str(contents.as_str());
            match new_paths_res {
                Ok(new_paths) => {
                    load_path_history(store, new_paths);
                }
                Err(e) => {
                    error!("Failed to parse the file {}: {}", yaml_file.display(), e);
                }
            }
        }
        Err(e) => {
            error!("Failed to read file {}: {}", yaml_file.display(), e);
        }
    }
}

fn load_path_history(store: Store, new_paths: Vec<Path>) {
    for entry in new_paths {
        match entry.date.parse::<u64>() {
            Ok(sec) => {
                let _ = store
                    .add_path_to_history(&entry.path, sec)
                    .map_err(|e| error!("{}", e));
            }
            Err(e) => {
                error!("{}", e);
            }
        }
    }
}

#[derive(Serialize, Deserialize, PartialEq, Debug)]
struct Shortcut {
    name: String,
    path: String,
    description: Option<String>,
}

pub(crate) fn load_shortcuts_from_yaml(store: Store, yaml_file: PathBuf) {
    if !yaml_file.exists() {
        error!("File {} does not exist", yaml_file.display());
        return;
    }
    match fs::read_to_string(&yaml_file) {
        Ok(contents) => {
            let new_shortcuts_res: Result<Vec<Shortcut>, serde_yaml::Error> =
                serde_yaml::from_str(contents.as_str());
            match new_shortcuts_res {
                Ok(shortcuts) => {
                    load_shortcuts(store, shortcuts);
                }
                Err(e) => {
                    error!("Failed to parse the file {}: {}", yaml_file.display(), e);
                }
            }
        }
        Err(e) => {
            error!("Failed to read file {}: {}", yaml_file.display(), e);
        }
    }
}

fn load_shortcuts(store: Store, new_paths: Vec<Shortcut>) {
    for entry in new_paths {
        let _ = store.delete_shortcut(&entry.name);
        let _ = store
            .add_shortcut(&entry.name, &entry.path, entry.description.as_deref())
            .map_err(|e| error!("{}", e));
    }
}

/// Export paths from the paths table to a YAML file.
/// The YAML file will contain a list of objects with `date` and `path` fields.
/// The `date` field is a string representing a UNIX timestamp in seconds.
pub(crate) fn export_paths_to_yaml(store: Store, yaml_file: PathBuf) {
    match store.list_all_paths() {
        Ok(paths) => {
            let export_paths: Vec<Path> = paths
                .into_iter()
                .map(|p| Path {
                    date: p.date.to_string(),
                    path: p.path,
                })
                .collect();

            match serde_yaml::to_string(&export_paths) {
                Ok(yaml_content) => {
                    if let Err(e) = fs::write(&yaml_file, yaml_content) {
                        error!("Failed to write file {}: {}", yaml_file.display(), e);
                    } else {
                        info!(
                            "Exported {} current paths to {}",
                            export_paths.len(),
                            yaml_file.display()
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to serialize paths to YAML: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to retrieve current paths: {}", e);
        }
    }
}

/// Export path history from the paths_history table to a YAML file.
/// The YAML file will contain a list of objects with `date` and `path` fields.
/// The `date` field is a string representing a UNIX timestamp in seconds.
pub(crate) fn export_path_history_to_yaml(store: Store, yaml_file: PathBuf) {
    match store.list_all_path_history() {
        Ok(paths) => {
            let export_paths: Vec<Path> = paths
                .into_iter()
                .map(|p| Path {
                    date: p.date.to_string(),
                    path: p.path,
                })
                .collect();

            match serde_yaml::to_string(&export_paths) {
                Ok(yaml_content) => {
                    if let Err(e) = fs::write(&yaml_file, yaml_content) {
                        error!("Failed to write file {}: {}", yaml_file.display(), e);
                    } else {
                        info!(
                            "Exported {} path history entries to {}",
                            export_paths.len(),
                            yaml_file.display()
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to serialize paths to YAML: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to retrieve path history: {}", e);
        }
    }
}

/// Export shortcuts to a YAML file.
/// The YAML file will contain a list of objects with `name`, `path`, and optional `description` fields.
pub(crate) fn export_shortcuts_to_yaml(store: Store, yaml_file: PathBuf) {
    match store.list_all_shortcuts() {
        Ok(shortcuts) => {
            let export_shortcuts: Vec<Shortcut> = shortcuts
                .into_iter()
                .map(|s| Shortcut {
                    name: s.name,
                    path: s.path,
                    description: s.description,
                })
                .collect();

            match serde_yaml::to_string(&export_shortcuts) {
                Ok(yaml_content) => {
                    if let Err(e) = fs::write(&yaml_file, yaml_content) {
                        error!("Failed to write file {}: {}", yaml_file.display(), e);
                    } else {
                        info!(
                            "Exported {} shortcuts to {}",
                            export_shortcuts.len(),
                            yaml_file.display()
                        );
                    }
                }
                Err(e) => {
                    error!("Failed to serialize shortcuts to YAML: {}", e);
                }
            }
        }
        Err(e) => {
            error!("Failed to retrieve shortcuts: {}", e);
        }
    }
}
