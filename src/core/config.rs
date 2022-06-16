use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub plugins: HashMap<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GeneralConfig {
    /// A tuple of (Modifier, Key)
    pub hotkey: (String, String),
    /// A vector of glob patterns
    pub blacklist: Vec<String>,
    pub max_search_results: u32,
}
#[derive(Serialize, Deserialize, Debug)]
pub struct AppearanceConfig {
    pub window_width: u32,
    pub input_height: u32,
    pub results_height: u32,
    pub results_item_height: u32,
    pub transparent: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            general: GeneralConfig {
                hotkey: ("AltLeft".into(), "Space".into()),
                blacklist: Vec::new(),
                max_search_results: 24,
            },
            appearance: AppearanceConfig {
                window_width: 600,
                input_height: 60,
                results_height: 480,
                results_item_height: 60,
                transparent: true,
            },
            plugins: HashMap::new(),
        }
    }
}

const CONFIG_FILE_NAME: &str = "kal.conf.json";

impl Config {
    /// Loads the config from the conventional location `$HOME/.kal/kal.conf.json`
    pub fn load() -> Result<Config> {
        let mut path = dirs_next::home_dir().context("Failed to get $home_dir path")?;
        path.push(".config");
        path.push(CONFIG_FILE_NAME);
        Self::load_from_path(path)
    }

    /// Loads the config from a path
    pub fn load_from_path<P: AsRef<path::Path>>(path: P) -> Result<Config> {
        let path = path.as_ref();
        let config;
        if path.exists() {
            let config_json = fs::read_to_string(path)?;
            config = serde_json::from_str::<Config>(&config_json)?;
        } else {
            config = Config::default();
            fs::create_dir_all(
                path.parent()
                    .context("Failed to get config file parent dir")?,
            )?;
            fs::write(path, serde_json::to_string_pretty(&config)?.as_bytes())?;
        }

        Ok(config)
    }

    /// Gets the specified plugin config
    pub fn plugin_config<T>(&self, name: &str) -> Option<T>
    where
        for<'de> T: Deserialize<'de>,
    {
        self.plugins
            .get(name)
            .map(|c| serde_json::from_value(c.clone()).unwrap())
    }
}
