use crate::tableview::Colors;
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::Write;
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
    fn build_config_file_path(config_file_path: Option<PathBuf>) -> PathBuf {
        if config_file_path.is_some() {
            config_file_path.unwrap()
        } else if let Ok(config_file_path) = env::var(CDIR_CONFIG_VAR) {
            PathBuf::from(config_file_path)
        } else {
            Self::build_default_config_path()
        }
    }

    fn build_default_config_path() -> PathBuf {
        let mut path = dirs::home_dir().unwrap();
        path.push(".config");
        path.push("cdir");
        path.push("config.yaml");
        return path;
    }

    pub fn load(config_file_path: Option<PathBuf>) -> Result<Config, String> {
        let path = Self::build_config_file_path(config_file_path);

        if !path.exists() {
            Self::initialize(path.clone());
        }

        let file = std::fs::File::open(path.clone());
        let mut config: Config;

        match serde_yaml::from_reader(file.unwrap()) {
            Ok(c) => {
                config = c;
            }
            Err(e) => {
                return Err(format!("Failed to parse config file {:?}: {}", path, e));
            }
        }

        let date_format = config.date_format.clone();
        config.date_formater = Box::from(move |s: i64| {
            DateTime::from_timestamp(s, 0)
                .unwrap()
                .with_timezone(&Local::now().timezone())
                .format(date_format.as_str())
                .to_string()
        });

        Ok(config)
    }

    fn initialize(config_file_path: PathBuf) {
        if config_file_path.exists() {
            panic!(
                "Error: Configuration file {:?} already exists",
                config_file_path
            );
        }

        println!("→ Initializing the configuration...");

        println!(
            "→ Creating the default configuration file {:?}",
            config_file_path
        );

        let config_dir = config_file_path.parent().unwrap();
        let data_dir = dirs::data_dir().unwrap().join("cdir");

        // ensure the data directory exists
        println!("→ Creating data directory {:?}", data_dir);
        fs::create_dir_all(&data_dir)
            .expect(&format!("Failed to create data directory {:?}", data_dir));

        // ensure the config directory exists
        fs::create_dir_all(config_dir).expect(&format!(
            "Failed to create config directory {:?}",
            config_dir
        ));

        // de-templatize and write the config file
        let config_template = include_str!("../templates/config.yaml");
        let config = config_template
            .replace("__CONFIG_PATH__", config_dir.to_str().unwrap())
            .replace("__DATA_PATH__", data_dir.to_str().unwrap());
        fs::write(&config_file_path, config).expect(&format!(
            "Failed to write config file {:?}",
            config_file_path
        ));

        // de-templatize and write the log4rs file if it doesn't exist
        let log4rs_config_path = config_dir.join("log4rs.yaml");
        if !log4rs_config_path.exists() {
            let log4rs_template = include_str!("../templates/log4rs.yaml");
            let log4rs_config = log4rs_template
                .replace("__CONFIG_PATH__", config_dir.to_str().unwrap())
                .replace("__DATA_PATH__", data_dir.to_str().unwrap());

            println!("→ Creating the log4rs config file {:?}", log4rs_config_path);
            fs::write(&log4rs_config_path, log4rs_config).expect(&format!(
                "Failed to write log4rs config file {:?}",
                log4rs_config_path
            ));
        }

        // create the .cdirsh file in the home directory
        let cdirsh_path = dirs::home_dir().unwrap().join(".cdirsh");
        let mut cdirsh_content =
            String::from("# cdir shell configuration\n# Do not edit this file manually.\n");
        // get the path to the current binary and add it to the PATH
        let exe_path = env::current_exe().unwrap();
        cdirsh_content.push_str(&format!(
            "export PATH=\"$PATH:{}\"\n",
            exe_path.parent().unwrap().to_str().unwrap()
        ));
        // set the CDIR_CONFIG environment variable if not using the default path
        if config_file_path != Self::build_default_config_path() {
            cdirsh_content.push_str(&format!(
                "export CDIR_CONFIG={}\n",
                config_file_path.to_str().unwrap()
            ));
        }
        // source the cdir_funcs.sh file
        cdirsh_content.push_str("source cdir_funcs.sh\n");

        println!("→ Creating the cdir shell file {:?}", cdirsh_path);

        // write the .cdirsh file
        fs::write(&cdirsh_path, cdirsh_content)
            .expect(&format!("Failed to write cdirsh file {:?}", cdirsh_path));

        // Ensure .cdirsh is sourced in .bashrc and .zshrc
        for shellrc_name in [".bashrc", ".zshrc"] {
            let shellrc = dirs::home_dir().unwrap().join(shellrc_name);
            let source_line = format!("source {}\n", cdirsh_path.to_str().unwrap());
            let mut needs_source = false;
            if shellrc.exists() {
                let content = fs::read_to_string(&shellrc)
                    .expect(&format!("Failed to read shell rc file {:?}", shellrc));
                if !content.contains(&source_line) {
                    needs_source = true;
                }
            }
            if needs_source {
                println!("→ Adding source line to {:?}", shellrc);
                let mut file = fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&shellrc)
                    .expect(&format!("Failed to open shell rc file {:?}", shellrc));
                file.write_all(source_line.as_bytes())
                    .expect(&format!("Failed to write to shell rc file {:?}", shellrc));
            }
        }

        println!("✓ Configuration is ready. Please restart your shell or run 'source ~/.cdirsh' to apply the changes.");
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            colors: Default::default(),
            date_formater: Box::new(|date| date.to_string()),
            db_path: Default::default(),
            log_config_path: Default::default(),
            date_format: Default::default(),
        }
    }
}

// Implement Clone manually for Config due to the closure field
impl Clone for Config {
    fn clone(&self) -> Self {
        Config {
            db_path: self.db_path.clone(),
            log_config_path: self.log_config_path.clone(),
            date_format: self.date_format.clone(),
            colors: self.colors.clone(),
            // Provide a new default closure for date_formater
            date_formater: Box::new(|date| date.to_string()),
        }
    }
}
