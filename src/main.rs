mod config;
mod config_button;
mod config_view;
mod confirmation;
mod expimp;
mod gui;
mod help;
mod history_view_container;
mod list_indicator_view;
mod model;
mod search_text_view;
mod shortcut_editor;
mod shortcut_view_container;
mod store;
mod tableview;
mod text_to_ansi;
mod theme;
mod tui;

use std::{
    error::Error,
    fs::File,
    io::Write,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use clap::{Parser, Subcommand};
use config::Config;
use expimp::load_paths_from_yaml;
use log::{debug, error, info};
use ratatui::text::Text;
use store::Store;

use crate::{expimp::load_shortcuts_from_yaml, store::Shortcut, text_to_ansi::text_to_ansi};

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
    /// Pretty print a path using shortcuts
    PrettyPrintPath {
        /// the path to pretty print
        path: String,
        /// whether to apply style (default is true)
        style: Option<bool>,
        /// if set, the maximum width of the string
        max_width: Option<u16>,
    },
}

fn initialize_logs(config_path: &Option<PathBuf>) {
    if let Some(config_path) = config_path.as_ref()
        && config_path.exists()
    {
        let _ = log4rs::init_file(config_path, Default::default());
    };
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    color_eyre::install()?;
    let args = Args::parse();
    let mut config = match Config::load(args.config_file.clone()) {
        Ok(config) => config,
        Err(e) => {
            error!("{}", e);
            return Err(Box::<dyn Error>::from(e));
        }
    };
    initialize_logs(&config.log_config_path);
    config.process();

    info!("Starting with args={args:?}");

    let config = Arc::new(Mutex::new(config));

    let store = Store::new(
        config
            .lock()
            .unwrap()
            .db_path
            .as_ref()
            .expect("missing db_path into the configuration"),
        config.clone(),
    );
    match &args.command {
        Some(Commands::Gui { filename }) => {
            if let Some(s) = gui::gui(store, config.clone()).await {
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
            debug!("AddShortcut {} {} {:?}", name, path, description);
            store
                .add_shortcut(name, path, description.as_ref().map(|s| s.as_str()))
                .unwrap()
        }
        Some(Commands::DeleteShortcut { name }) => {
            debug!("DeleteShortcut {}", name);
            store.delete_shortcut(name).unwrap();
        }
        Some(Commands::PrintShortcut { name }) => {
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
            let list = store.list_paths(0, 10, "", false).unwrap();
            let config_lock = config.lock().unwrap();
            list.iter()
                .for_each(|s| println!("{} {}", (config_lock.date_formater)(s.date), s.path));
        }
        Some(Commands::PrettyPrintPath {
            path,
            style,
            max_width,
        }) => {
            let max_width = max_width.unwrap_or(u16::MAX);
            let shortcuts: Vec<Shortcut> = store.list_all_shortcuts().unwrap();
            let config_lock = config.lock().unwrap();
            let shortened_line =
                gui::Gui::shorten_path_for_shortcut(&config_lock, &shortcuts, path, max_width);
            let shortened_line = shortened_line
                .unwrap_or_else(|| {
                    gui::Gui::reduce_path(
                        path.clone(),
                        max_width,
                        config_lock.styles.home_tilde_style,
                    )
                })
                .style(config_lock.styles.path_style);
            if style.is_none_or(|s| s) {
                print!("{}", text_to_ansi(&Text::from(shortened_line)));
            } else {
                print!("{}", shortened_line);
            }
        }
        None => {
            println!("Use the 'c' shell alias to launch the GUI.");
            println!("Use --help to see available commands.");
            println!("Documentation is available at https://github.com/AmadeusITGroup/cdir");
        }
    }

    Ok(())
}
