use std::{
    env, fs,
    io::Write,
    path::PathBuf,
    sync::{Arc, OnceLock},
};

use chrono::{DateTime, Local};
use log::{debug, error, info, trace};
use serde::{Deserialize, Serialize};
use serde_yaml::Value;
use yamlpatch::{Op, Patch, apply_yaml_patches};
use yamlpath::route;

use crate::theme::{Theme, ThemeStyles};

pub(crate) const CDIR_CONFIG_VAR: &str = "CDIR_CONFIG";

static CONFIG_FILE_PATH: OnceLock<PathBuf> = OnceLock::new();

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

const DEFAULT_SMART_SUGGESTIONS_DEPTH: fn() -> usize = || 5;

const DEFAULT_SMART_SUGGESTIONS_COUNT: fn() -> usize = || 3;

const DEFAULT_THEMES_DIRECTORY_PATH: fn() -> Option<PathBuf> = || {
    let mut path = dirs::home_dir().unwrap();
    path.push(".config");
    path.push("cdir");
    path.push("themes");
    Some(path)
};

const DEFAULT_DATE_FORMAT: fn() -> String = || String::from("%d-%b-%y %H:%M:%S");

const DEFAULT_THEME: fn() -> Option<String> = || Some(String::from("default"));

const DEFAULT_COLORS: fn() -> Theme = || serde_yaml::from_str("").unwrap();

const DEFAULT_DATE_FORMATER: fn() -> Arc<dyn Fn(i64) -> String + Send + Sync> =
    || Arc::from(|_| String::from(""));

const DEFAULT_NONE: fn() -> Option<String> = || None;

const DEFAULT_TRUE: fn() -> bool = || true;

/// Application configuration structure.
/// The configuration can be loaded from a YAML file.
#[derive(Serialize, Deserialize)]
pub struct Config {
    #[serde(default = "DEFAULT_DB_PATH")]
    pub db_path: Option<PathBuf>,

    #[serde(default = "DEFAULT_LOG_CONFIG_PATH")]
    pub log_config_path: Option<PathBuf>,

    #[serde(default = "DEFAULT_TRUE")]
    pub path_search_include_shortcuts: bool,

    #[serde(default = "DEFAULT_TRUE")]
    pub smart_suggestions_active: bool,

    #[serde(default = "DEFAULT_SMART_SUGGESTIONS_DEPTH")]
    pub smart_suggestions_depth: usize,

    #[serde(default = "DEFAULT_SMART_SUGGESTIONS_COUNT")]
    pub smart_suggestions_count: usize,

    #[serde(default = "DEFAULT_THEMES_DIRECTORY_PATH")]
    pub themes_directory_path: Option<PathBuf>,

    #[serde(default = "DEFAULT_DATE_FORMAT")]
    pub date_format: String,

    #[serde(default = "DEFAULT_THEME")]
    pub theme: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub theme_dark: Option<String>,

    #[serde(default = "DEFAULT_NONE")]
    pub theme_light: Option<String>,

    #[serde(default = "DEFAULT_COLORS")]
    pub inline_theme: Theme,

    #[serde(default = "DEFAULT_COLORS")]
    pub inline_theme_dark: Theme,

    #[serde(default = "DEFAULT_COLORS")]
    pub inline_theme_light: Theme,

    #[serde(skip_serializing)]
    #[serde(skip_deserializing)]
    pub styles: ThemeStyles,

    #[serde(skip, default = "DEFAULT_DATE_FORMATER")]
    pub date_formater: Arc<dyn Fn(i64) -> String + Send + Sync>,
}

impl Config {
    fn build_config_file_path(config_file_path: Option<PathBuf>) -> PathBuf {
        if let Some(path) = config_file_path {
            path
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
        path
    }

    pub fn initialize_and_load(config_file_path: Option<PathBuf>) -> Result<Config, String> {
        let path = Self::build_config_file_path(config_file_path);

        if !path.exists() {
            Self::initialize(path.clone());
            Self::install_themes(path.clone());
        }

        CONFIG_FILE_PATH.get_or_init(|| path.clone());

        Self::load(path)
    }

    pub fn load(path: PathBuf) -> Result<Config, String> {
        let file = std::fs::File::open(path.clone());

        match serde_yaml::from_reader(file.unwrap()) {
            Ok(c) => Ok(c),
            Err(e) => Err(format!("Failed to parse config file {:?}: {}", path, e)),
        }
    }

    pub fn process(self: &mut Config) -> &Config {
        let actual_theme = Self::process_themes(self);

        // compute the styles fom the current inline_theme
        self.styles = ThemeStyles::from(&actual_theme);

        let date_format = self.date_format.clone();
        self.date_formater = Arc::from(move |s: i64| {
            DateTime::from_timestamp(s, 0)
                .unwrap()
                .with_timezone(&Local::now().timezone())
                .format(date_format.as_str())
                .to_string()
        });

        self
    }

    fn process_themes(config: &Config) -> Theme {
        let actual_theme: Theme;
        let mut external_theme = Theme::default();

        // Process the dark/light config
        if config.theme_dark.is_some() && config.theme_light.is_some() {
            let dl = match dark_light::detect() {
                Ok(dl) => dl,
                Err(err) => {
                    error!("failed to detect dark_light mode {}", err);
                    return Self::build_regular_theme(config);
                }
            };
            match dl {
                dark_light::Mode::Dark => {
                    trace!("dark mode detected");
                    if let Some(theme) = config.load_theme(config.theme_dark.as_ref().unwrap()) {
                        external_theme = theme.merge(&external_theme);
                    }
                    actual_theme = config.inline_theme_dark.merge(&external_theme);
                }
                dark_light::Mode::Light => {
                    trace!("light mode detected");
                    if let Some(theme) = config.load_theme(config.theme_light.as_ref().unwrap()) {
                        external_theme = theme.merge(&external_theme);
                    }
                    actual_theme = config.inline_theme_light.merge(&external_theme);
                }
                dark_light::Mode::Unspecified => {
                    info!("dark/light mode not detected");
                    return Self::build_regular_theme(config);
                }
            }
        } else {
            // process the regular theme setup
            return Self::build_regular_theme(config);
        }
        actual_theme
    }

    fn build_regular_theme(config: &Config) -> Theme {
        let mut external_theme = Theme::default();
        if let Some(theme) = config.theme.as_ref()
            && let Some(theme) = config.load_theme(theme)
        {
            external_theme = theme.merge(&external_theme);
        }
        config.inline_theme.merge(&external_theme)
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
            .unwrap_or_else(|_| panic!("Failed to create data directory {:?}", data_dir));

        // ensure the config directory exists
        fs::create_dir_all(config_dir)
            .unwrap_or_else(|_| panic!("Failed to create config directory {:?}", config_dir));

        // de-templatize and write the config file
        let config_template = include_str!("../templates/config.yaml");
        let config = config_template
            .replace("__CONFIG_PATH__", config_dir.to_str().unwrap())
            .replace("__DATA_PATH__", data_dir.to_str().unwrap());
        fs::write(&config_file_path, config)
            .unwrap_or_else(|_| panic!("Failed to write config file {:?}", config_file_path));

        // de-templatize and write the log4rs file if it doesn't exist
        let log4rs_config_path = config_dir.join("log4rs.yaml");
        if !log4rs_config_path.exists() {
            let log4rs_template = include_str!("../templates/log4rs.yaml");
            let log4rs_config = log4rs_template
                .replace("__CONFIG_PATH__", config_dir.to_str().unwrap())
                .replace("__DATA_PATH__", data_dir.to_str().unwrap());

            println!("→ Creating the log4rs config file {:?}", log4rs_config_path);
            fs::write(&log4rs_config_path, log4rs_config).unwrap_or_else(|_| {
                panic!(
                    "Failed to write log4rs config file {:?}",
                    log4rs_config_path
                )
            });
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
            .unwrap_or_else(|_| panic!("Failed to write cdirsh file {:?}", cdirsh_path));

        // Ensure .cdirsh is sourced in .bashrc and .zshrc
        for shellrc_name in [".bashrc", ".zshrc"] {
            let shellrc = dirs::home_dir().unwrap().join(shellrc_name);
            let source_line = format!("source {}\n", cdirsh_path.to_str().unwrap());
            let mut needs_source = false;
            if shellrc.exists() {
                let content = fs::read_to_string(&shellrc)
                    .unwrap_or_else(|_| panic!("Failed to read shell rc file {:?}", shellrc));
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
                    .unwrap_or_else(|_| panic!("Failed to open shell rc file {:?}", shellrc));
                file.write_all(source_line.as_bytes())
                    .unwrap_or_else(|_| panic!("Failed to write to shell rc file {:?}", shellrc));
            }
        }

        println!(
            "✓ Configuration is ready. Please restart your shell or run 'source ~/.cdirsh' to apply the changes."
        );
    }

    fn load_theme(&self, theme: &str) -> Option<Theme> {
        debug!("load_theme: {theme}");
        let themes_directory_path = match self.themes_directory_path.as_ref() {
            Some(tp) => tp,
            None => {
                error!("Theme directory not defined");
                panic!("Theme directory not defined");
            }
        };

        let theme_path = if themes_directory_path
            .join(String::from(theme) + ".yml")
            .exists()
        {
            themes_directory_path.join(String::from(theme) + ".yml")
        } else {
            themes_directory_path.join(String::from(theme) + ".yaml")
        };

        let file = match std::fs::File::open(&theme_path) {
            Ok(file) => file,
            Err(err) => {
                error!("Theme not found {:?}:{err}", &theme_path);
                return None;
            }
        };

        match serde_yaml::from_reader(file) {
            Ok(theme) => theme,
            Err(err) => {
                error!("{:?}:{err}", &theme_path);
                None
            }
        }
    }

    fn install_themes(config_file_path: PathBuf) {
        let template_dir = config_file_path.parent().unwrap().join("themes");
        if !template_dir.exists() {
            std::fs::create_dir(template_dir.clone()).unwrap();
        }
        Self::install_theme(
            template_dir.join("dark-blue.yaml"),
            include_str!("../themes/dark-blue.yaml"),
        );
        Self::install_theme(
            template_dir.join("dark.yaml"),
            include_str!("../themes/dark.yaml"),
        );
        Self::install_theme(
            template_dir.join("default.yaml"),
            include_str!("../themes/default.yaml"),
        );
        Self::install_theme(
            template_dir.join("light-autumn.yaml"),
            include_str!("../themes/light-autumn.yaml"),
        );
        Self::install_theme(
            template_dir.join("light-joy.yaml"),
            include_str!("../themes/light-joy.yaml"),
        );
        Self::install_theme(
            template_dir.join("pure.yaml"),
            include_str!("../themes/pure.yaml"),
        );
        Self::install_theme(
            template_dir.join("winter.yaml"),
            include_str!("../themes/winter.yaml"),
        );
    }

    fn install_theme(theme_path: PathBuf, content: &str) {
        if theme_path.exists() {
            return;
        }
        std::fs::write(&theme_path, content)
            .unwrap_or_else(|_| panic!("Failed create the theme file {:?}", theme_path));
    }

    // Save the configuration by applying a patch to the existing config file, to preserve comments and formatting as much as possible
    pub(crate) fn save(&self) -> Result<(), String> {
        info!("Saving configuration to {:?}", CONFIG_FILE_PATH.get());

        let config_path = CONFIG_FILE_PATH
            .get()
            .ok_or_else(|| String::from("Config file path not initialized"))?
            .clone();
        let config_source = fs::read_to_string(&config_path)
            .map_err(|e| format!("Failed to read config file {:?}: {}", config_path, e))?;
        let document = yamlpath::Document::new(config_source.clone())
            .map_err(|e| format!("Failed to parse config file {:?}: {}", config_path, e))?;

        let config_from_file: Config = serde_yaml::from_str(&config_source)
            .map_err(|e| format!("Failed to parse config file {:?}: {}", config_path, e))?;

        let mut patches: Vec<Patch> = Vec::new();
        let mut add_or_replace = |key: &str, value: Value| {
            let key_owned = key.to_string();
            let key_route = route!(key_owned.clone());
            if document.query_exists(&key_route) {
                patches.push(Patch {
                    route: key_route,
                    operation: Op::Replace(value),
                });
            } else {
                patches.push(Patch {
                    route: route!(),
                    operation: Op::Add {
                        key: key_owned,
                        value,
                    },
                });
            }
        };

        if config_from_file.smart_suggestions_active != self.smart_suggestions_active {
            add_or_replace(
                "smart_suggestions_active",
                Value::Bool(self.smart_suggestions_active),
            );
        }

        if config_from_file.smart_suggestions_count != self.smart_suggestions_count {
            add_or_replace(
                "smart_suggestions_count",
                Value::Number(serde_yaml::Number::from(
                    self.smart_suggestions_count as u64,
                )),
            );
        }

        if config_from_file.smart_suggestions_depth != self.smart_suggestions_depth {
            add_or_replace(
                "smart_suggestions_depth",
                Value::Number(serde_yaml::Number::from(
                    self.smart_suggestions_depth as u64,
                )),
            );
        }

        if config_from_file.path_search_include_shortcuts != self.path_search_include_shortcuts {
            add_or_replace(
                "path_search_include_shortcuts",
                Value::Bool(self.path_search_include_shortcuts),
            );
        }

        if patches.is_empty() {
            return Ok(());
        }

        let updated_document = apply_yaml_patches(&document, &patches)
            .map_err(|e| format!("Failed to apply config patch {:?}: {}", config_path, e))?;
        let updated_source = updated_document.source().to_string();

        fs::write(&config_path, updated_source)
            .map_err(|e| format!("Failed to write config file {:?}: {}", config_path, e))?;

        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Config {
            theme: Default::default(),
            theme_dark: Default::default(),
            theme_light: Default::default(),
            inline_theme: Default::default(),
            inline_theme_dark: Default::default(),
            inline_theme_light: Default::default(),
            styles: Default::default(),
            smart_suggestions_active: true,
            smart_suggestions_depth: DEFAULT_SMART_SUGGESTIONS_DEPTH(),
            smart_suggestions_count: DEFAULT_SMART_SUGGESTIONS_COUNT(),
            themes_directory_path: Default::default(),
            date_formater: Arc::new(|date| date.to_string()),
            db_path: Default::default(),
            log_config_path: Default::default(),
            path_search_include_shortcuts: true,
            date_format: Default::default(),
        }
    }
}

// Implement Clone manually for Config due to the closure field
impl Clone for Config {
    fn clone(&self) -> Self {
        Config {
            theme: self.theme.clone(),
            theme_dark: self.theme_dark.clone(),
            theme_light: self.theme_light.clone(),
            inline_theme: self.inline_theme.clone(),
            inline_theme_dark: self.inline_theme_dark.clone(),
            inline_theme_light: self.inline_theme_light.clone(),
            styles: self.styles.clone(),
            smart_suggestions_active: self.smart_suggestions_active,
            smart_suggestions_depth: self.smart_suggestions_depth,
            smart_suggestions_count: self.smart_suggestions_count,
            themes_directory_path: self.themes_directory_path.clone(),
            db_path: self.db_path.clone(),
            log_config_path: self.log_config_path.clone(),
            path_search_include_shortcuts: self.path_search_include_shortcuts,
            date_format: self.date_format.clone(),
            // Provide a new default closure for date_formater
            date_formater: Arc::new(|date| date.to_string()),
        }
    }
}
