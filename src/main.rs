mod config;
mod expimp;
mod gui;
mod help;
mod model;
mod shortcut_editor;
mod store;
mod tableview;

use crate::expimp::load_shortcuts_from_yaml;
use clap::{Parser, Subcommand};
use config::Config;
use expimp::load_paths_from_yaml;
use log::{debug, error, info};
use std::error::Error;
use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use store::Store;

/// cdir helps you to switch quickly and easily between directories
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Path to the configuration file
    #[arg(short, long)]
    config_file: Option<std::path::PathBuf>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Launch the GUI
    Gui { filename: Option<String> },
    /// Print the path to the configuration file
    ConfigFile,
    /// Add a directory path
    AddPath { path: String },
    /// Import a path file
    ImportPaths { filename: String },
    /// Add a shortcut
    AddShortcut {
        name: String,
        path: String,
        description: Option<String>,
    },
    /// Delete a shortcut
    DeleteShortcut { name: String },
    /// Print a shortcut
    PrintShortcut { name: String },
    /// Import a shortcuts file
    ImportShortcuts { filename: String },
    /// Print last paths
    Lasts,
}

fn initialize_logs(config_path: &Option<PathBuf>) {
    if let Some(config_path) = config_path.as_ref() {
        if config_path.exists() {
            let _ = log4rs::init_file(config_path, Default::default());
        }
    };
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Args::parse();
    let config = match Config::load(args.config_file.clone()) {
        Ok(config) => config,
        Err(e) => {
            error!("{}", e);
            return Err(Box::<dyn Error>::from(e));
        }
    };
    initialize_logs(&config.log_config_path);

    info!("Starting with args={args:?}");

    let store = Store::new(config.db_path.as_ref().unwrap());
    match &args.command {
        Some(Commands::Gui { filename }) => {
            if let Some(s) = gui::gui(store, config) {
                match filename {
                    None => {
                        println!("{}", s);
                    }
                    Some(filename) => {
                        let path = Path::new(filename);
                        let mut file = File::create(path).unwrap();
                        file.write_all(s.as_bytes()).unwrap();
                    }
                }
            };
        }
        Some(Commands::ConfigFile) => {
            if let Some(config_file) = &args.config_file {
                println!("{}", config_file.display());
            } else if let Ok(config_file) = std::env::var(config::CDIR_CONFIG_VAR) {
                println!("{}", config_file);
            } else {
                let mut cpath = dirs::home_dir().unwrap();
                cpath.push(".config");
                cpath.push("cdir");
                cpath.push("config.yaml");
                println!("{}", cpath.display());
            }
        }
        Some(Commands::AddPath { path }) => {
            store.add_path(path).unwrap();
        }
        Some(Commands::ImportPaths { filename }) => {
            load_paths_from_yaml(store, PathBuf::from(filename));
        }
        Some(Commands::AddShortcut {
            name,
            path,
            description,
        }) => {
            //store.add_path(path).unwrap();
            debug!("AddShortcut {} {} {:?}", name, path, description);
            store
                .add_shortcut(name, path, description.as_ref().map(|s| s.as_str()))
                .unwrap()
        }
        Some(Commands::DeleteShortcut { name }) => {
            //store.add_path(path).unwrap();
            debug!("DeleteShortcut {}", name);
            store.delete_shortcut(name).unwrap();
        }
        Some(Commands::PrintShortcut { name }) => {
            //store.add_path(path).unwrap();
            debug!("PrintShortcut {}", name);
            match store.find_shortcut(name) {
                None => {}
                Some(s) => {
                    print!("{}", s.path)
                }
            };
        }
        Some(Commands::ImportShortcuts { filename }) => {
            load_shortcuts_from_yaml(store, PathBuf::from(filename));
        }
        Some(Commands::Lasts) => {
            let list = store.list_paths(0, 10, "").unwrap();
            list.iter()
                .for_each(|s| println!("{} {}", (config.date_formater)(s.date), s.path));
        }
        None => {
            println!("Use the 'c' shell alias to launch the GUI.");
            println!("Use --help to see available commands.");
            println!("Documentation is available at https://github.com/AmadeusITGroup/cdir");
        }
    }

    Ok(())
}
