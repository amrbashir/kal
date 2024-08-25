use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use crate::vibrancy::Vibrancy;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppearanceConfig {
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_input_height")]
    pub input_height: u32,
    #[serde(default = "default_results_height")]
    pub results_height: u32,
    #[serde(default = "default_results_divier")]
    pub results_divier: u32,
    #[serde(default = "default_results_padding")]
    pub results_padding: u32,
    #[serde(default = "default_results_row_height")]
    pub results_row_height: u32,
    #[serde(default = "default_results_row_gap")]
    pub results_row_gap: u32,
    #[serde(default = "default_true")]
    pub transparent: bool,
    #[serde(default = "default_true")]
    pub shadows: bool,
    #[serde(default = "default_vibrancy")]
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
fn default_results_divier() -> u32 {
    1
}
fn default_results_padding() -> u32 {
    16
}
fn default_results_row_height() -> u32 {
    60
}
fn default_results_row_gap() -> u32 {
    4
}
fn default_vibrancy() -> Option<Vibrancy> {
    Some(Vibrancy::Mica)
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
            results_divier: default_results_divier(),
            results_padding: default_results_padding(),
            results_row_height: default_results_row_height(),
            results_row_gap: default_results_row_gap(),
            transparent: true,
            shadows: true,
            vibrancy: default_vibrancy(),
            custom_css_file: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    /// A string of hotkey
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    /// A vector of glob patterns
    #[serde(default)]
    pub blacklist: Vec<String>,
    #[serde(default = "default_max_search_results")]
    pub max_search_results: usize,
}

fn default_hotkey() -> String {
    "Alt+Space".to_string()
}

fn default_max_search_results() -> usize {
    24
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            blacklist: Vec::new(),
            max_search_results: default_max_search_results(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct GenericPluginConfig {
    pub enabled: Option<bool>,
    pub include_in_global_results: Option<bool>,
    pub direct_activation_command: Option<String>,
}

impl GenericPluginConfig {
    pub fn apply_from(mut self, another: &Self) -> Self {
        if self.enabled.is_none() {
            self.enabled = another.enabled;
        }
        if self.include_in_global_results.is_none() {
            self.include_in_global_results = another.include_in_global_results;
        }
        if self.direct_activation_command.is_none() {
            self.direct_activation_command
                .clone_from(&another.direct_activation_command);
        }
        self
    }

    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    pub fn include_in_global_results(&self) -> bool {
        self.include_in_global_results.unwrap_or(true)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginConfig {
    #[serde(flatten, default)]
    pub generic: GenericPluginConfig,
    #[serde(flatten)]
    pub inner: Option<toml::Table>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Config {
    #[serde(default)]
    pub general: GeneralConfig,
    #[serde(default)]
    pub appearance: AppearanceConfig,
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
}

impl Config {
    /// Loads config from a toml string
    fn from_toml(toml: &str) -> Self {
        toml::from_str(toml)
            .inspect_err(|e| {
                tracing::error!("Failed to deserialize config, falling back to default: {e}")
            })
            .unwrap_or_default()
    }

    /// Loads config from a path
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
        tracing::info!("Config loaded: {config:?}");
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
            .and_then(|c| {
                c.inner.clone().and_then(|c| {
                    toml::Table::try_into(c)
                        .inspect_err(|e| {
                            tracing::error!(
                        "Failed to deserialize {name} config, failling back to default: {e}"
                    );
                        })
                        .ok()
                })
            })
            .unwrap_or_default()
    }

    pub fn generic_config(&self, name: &str) -> Option<GenericPluginConfig> {
        self.plugins.get(name).map(|c| c.generic.clone())
    }
}
