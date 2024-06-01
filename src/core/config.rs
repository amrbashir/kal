use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::{vibrancy::Vibrancy, CONFIG_FILE};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppearanceConfig {
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_input_height")]
    pub input_height: u32,
    #[serde(default = "default_results_height")]
    pub results_height: u32,
    #[serde(default = "default_results_item_height")]
    pub results_item_height: u32,
    #[serde(default)]
    pub transparent: bool,
    #[serde(default = "default_true")]
    pub shadows: bool,
    pub vibrancy: Option<Vibrancy>,
    pub custom_css_file: Option<PathBuf>,
}

fn default_window_width() -> u32 {
    600
}
fn default_input_height() -> u32 {
    60
}
fn default_results_height() -> u32 {
    480
}
fn default_results_item_height() -> u32 {
    60
}
fn default_true() -> bool {
    true
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            window_width: default_window_width(),
            input_height: default_input_height(),
            results_height: default_results_height(),
            results_item_height: default_results_item_height(),
            transparent: true,
            shadows: true,
            vibrancy: None,
            custom_css_file: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    /// A string of hotkey
    #[serde(default)]
    pub hotkey: String,
    /// A vector of glob patterns
    #[serde(default)]
    pub blacklist: Vec<String>,
    #[serde(default)]
    pub max_search_results: usize,
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            hotkey: "Alt+Space".into(),
            blacklist: Vec::new(),
            max_search_results: 24,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub plugins: HashMap<String, toml::Table>,
}

impl Config {
    /// Loads the config from the conventional location `$HOME/.config/kal.conf.json`
    pub fn load() -> anyhow::Result<Config> {
        Self::load_from_path(&*CONFIG_FILE)
    }

    fn from_toml(toml: &str) -> Self {
        toml::from_str(toml)
            .inspect_err(|e| {
                tracing::error!("Failed to deserialize config, falling back to default: {e}")
            })
            .unwrap_or_default()
    }

    /// Loads the config from a path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> anyhow::Result<Config> {
        let path = path.as_ref();
        let config = match path.exists() {
            true => {
                let toml = fs::read_to_string(path)?;
                Self::from_toml(&toml)
            }
            false => {
                tracing::error!("Config file wasn't found, falling back to default");
                Config::default()
            }
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
        self.plugins
            .get(name)
            .and_then(|toml_value| {
                toml::from_str(&toml_value.to_string())
                    .inspect_err(|e| {
                        tracing::error!(
                            "Failed to deserialize {name} config, failling back to default: {e}"
                        );
                    })
                    .ok()
            })
            .unwrap_or_default()
    }
}
