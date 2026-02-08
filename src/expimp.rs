use std::{fs, path::PathBuf};

use log::error;
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
