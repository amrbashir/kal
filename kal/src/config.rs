use std::collections::HashMap;
use std::path::PathBuf;

use anyhow::Context;
use serde::{Deserialize, Serialize};

use crate::webview_window::Vibrancy;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppearanceConfig {
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    #[serde(default = "default_input_height")]
    pub input_height: u32,
    #[serde(default = "default_input_items_gap")]
    pub input_items_gap: u32,
    #[serde(default = "default_max_items")]
    pub max_items: u32,
    #[serde(default = "default_item_height")]
    pub item_height: u32,
    #[serde(default = "default_item_gap")]
    pub item_gap: u32,
    #[serde(default = "default_true")]
    pub transparent: bool,
    #[serde(default = "default_true")]
    pub shadows: bool,
    #[serde(default = "default_vibrancy")]
    pub vibrancy: Option<Vibrancy>,
    pub custom_css_file: Option<PathBuf>,
}

fn default_window_width() -> u32 {
    650
}
fn default_input_height() -> u32 {
    65
}
fn default_input_items_gap() -> u32 {
    16
}
fn default_max_items() -> u32 {
    8
}
fn default_item_height() -> u32 {
    55
}
fn default_item_gap() -> u32 {
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
            input_items_gap: default_input_items_gap(),
            max_items: default_max_items(),
            item_height: default_item_height(),
            item_gap: default_item_gap(),
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
    #[serde(default = "default_tab_through_context_buttons")]
    pub tab_through_context_buttons: bool,
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_hotkey() -> String {
    "Alt+Space".to_string()
}
fn default_tab_through_context_buttons() -> bool {
    true
}
fn default_max_results() -> usize {
    24
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            tab_through_context_buttons: default_tab_through_context_buttons(),
            max_results: default_max_results(),
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

    /// Default config path:
    /// - `debug`: `$CWD/kal.toml`
    /// - `release`: `$HOME/.config/kal.toml`
    pub fn path() -> anyhow::Result<PathBuf> {
        #[cfg(debug_assertions)]
        return std::env::current_dir()
            .context("Failed to get current directory path")
            .map(|p| p.join("kal.toml"));

        #[cfg(not(debug_assertions))]
        dirs::home_dir()
            .context("Failed to get $HOME dir path")
            .map(|p| p.join(".config").join("kal.toml"))
    }

    /// Asyncly loads config from a canonical path
    pub async fn load_async() -> anyhow::Result<Self> {
        let span = tracing::info_span!("config::load");
        let _enter = span.enter();

        let path = Self::path()?;
        let config = match path.exists() {
            true => {
                let toml = smol::fs::read_to_string(path).await?;
                Self::from_toml(&toml)
            }
            false => {
                tracing::warn!("Config file wasn't found, falling back to default");
                Config::default()
            }
        };

        Ok(config)
    }

    /// Loads config from a canonical path
    pub fn load() -> anyhow::Result<Self> {
        let span = tracing::info_span!("config::load");
        let _enter = span.enter();

        let path = Self::path()?;
        let config = match path.exists() {
            true => {
                let toml = std::fs::read_to_string(path)?;
                Self::from_toml(&toml)
            }
            false => {
                tracing::warn!("Config file wasn't found, falling back to default");
                Config::default()
            }
        };

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
            .and_then(|c| c.inner.clone())
            .and_then(|c| {
                toml::Table::try_into(c)
                    .inspect_err(|e| {
                        tracing::error!(
                            "Failed to deserialize {name} config, failling back to default: {e}"
                        );
                    })
                    .ok()
            })
            .unwrap_or_default()
    }

    pub fn generic_config_for_plugin(&self, name: &str) -> Option<GenericPluginConfig> {
        self.plugins.get(name).map(|c| c.generic.clone())
    }
}
