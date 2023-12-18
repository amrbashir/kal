use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fs, path::Path};

use crate::CONFIG_FILE;

#[derive(Deserialize, Debug, Clone)]
pub struct Config {
    pub general: GeneralConfig,
    pub appearance: AppearanceConfig,
    pub plugins: HashMap<String, toml::Table>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    /// A tuple of (Modifier, Key)
    pub hotkey: (String, String),
    /// A vector of glob patterns
    pub blacklist: Vec<String>,
    pub max_search_results: u32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
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

impl Config {
    /// Loads the config from the conventional location `$HOME/.config/kal.conf.json`
    pub fn load() -> anyhow::Result<Config> {
        Self::load_from_path(&*CONFIG_FILE)
    }

    /// Loads the config from a path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
        let path = path.as_ref();
        let config = if path.exists() {
            let config_json = fs::read_to_string(path)?;
            match toml::from_str::<Config>(&config_json) {
                Ok(c) => c,
                Err(e) => {
                    tracing::error!("Failed to deserialize config, falling back to default: {e}",);
                    Config::default()
                }
            }
        } else {
            tracing::debug!("Config file wasn't found, falling back to default");
            Config::default()
        };
        tracing::info!("Config loaded");
        Ok(config)
    }

    /// Gets the specified plugin config
    pub fn plugin_config<T>(&self, name: &str) -> T
    where
        T: Default,
        for<'de> T: Deserialize<'de>,
    {
        if let Some(toml_value) = self.plugins.get(name) {
            toml::from_str(&toml_value.to_string()).unwrap_or_else(|e| {
                tracing::error!(
                    "Failed to deserialize {name} config, failling back to default: {e}"
                );
                T::default()
            })
        } else {
            T::default()
        }
    }
}
