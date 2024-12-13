use crate::tableview::Colors;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::env;
use std::path::PathBuf;

pub(crate) const CDIR_CONFIG_VAR: &str = "CDIR_CONFIG";

const DEFAULT_DB_PATH: fn() -> Option<PathBuf> = || {
    let mut path = dirs::data_dir().unwrap();
    path.push("cdir");
    path.push("cdir.db");
    Some(path)
};

const DEFAULT_LOG_CONFIG_PATH: fn() -> Option<PathBuf> = || {
    let mut path = dirs::home_dir().unwrap();
    path.push(".config");
    path.push("cdir");
    path.push("log4rs.yaml");
    Some(path)
};

const DEFAULT_DATE_FORMAT: fn() -> String = || String::from("%d-%b-%y %H:%M:%S");

const DEFAULT_COLORS: fn() -> Colors = || serde_yaml::from_str("").unwrap();

const DEFAULT_DATE_FORMATER: fn() -> Box<dyn Fn(i64) -> String> =
    || Box::from(|_| String::from(""));

/// Application configuration structure.
/// The configuration can be loaded from a YAML file.
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "DEFAULT_DB_PATH")]
    pub db_path: Option<PathBuf>,

    #[serde(default = "DEFAULT_LOG_CONFIG_PATH")]
    pub log_config_path: Option<PathBuf>,

    #[serde(default = "DEFAULT_DATE_FORMAT")]
    pub date_format: String,

    #[serde(default = "DEFAULT_COLORS")]
    pub colors: Colors,

    #[serde(skip, default = "DEFAULT_DATE_FORMATER")]
    pub date_formater: Box<dyn Fn(i64) -> String>,
}

impl Config {
    pub fn new() -> Config {
        serde_yaml::from_str("").unwrap()
    }

    pub fn load(config_file_path: &Option<PathBuf>) -> Result<Config, serde_yaml::Error> {
        let mut path: Option<PathBuf> = None;
        if config_file_path.is_some() {
            path = config_file_path.clone();
        } else if let Ok(config_file_path) = env::var(CDIR_CONFIG_VAR) {
            path = Some(PathBuf::from(config_file_path));
        } else {
            let mut cpath = dirs::home_dir().unwrap();
            cpath.push(".config");
            cpath.push("cdir");
            cpath.push("config.yaml");
            if cpath.exists() {
                path = Some(cpath);
            }
        }

        let config_result = path.map_or(Ok(Config::new()), |config_file_path| {
            if !config_file_path.exists() {
                eprint!("Configuration file '{:?}' not found. Please set the {} environment variable or create a config.yaml file in ~/.config/cdir/", config_file_path, CDIR_CONFIG_VAR);
            }
            let file = std::fs::File::open(config_file_path);
            serde_yaml::from_reader(file.unwrap())
        });

        config_result.map(|mut config| {
            let date_format = config.date_format.clone();
            config.date_formater = Box::from(move |s: i64| {
                DateTime::from_timestamp(s, 0)
                    .unwrap()
                    .with_timezone(&Local::now().timezone())
                    .format(date_format.as_str())
                    .to_string()
            });
            config
        })
    }
}
