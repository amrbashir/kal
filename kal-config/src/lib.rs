use std::collections::HashMap;
use std::path::{Path, PathBuf};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

mod appearance;
mod error;
mod general;
mod plugin;

pub use appearance::*;
pub use error::*;
pub use general::*;
pub use plugin::*;

/// Kal configuration.
#[derive(Serialize, Deserialize, Debug, Clone, Default, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// General configuration.
    #[serde(default)]
    pub general: GeneralConfig,
    /// Appearance configuration.
    #[serde(default)]
    pub appearance: AppearanceConfig,
    /// Plugins configuration.
    #[serde(default)]
    pub plugins: HashMap<String, PluginConfig>,
}

impl Config {
    /// Default config path:
    /// - `debug`: `$CWD/kal.toml`
    /// - `release`: `$HOME/.config/kal.toml`
    pub fn path() -> Result<PathBuf> {
        #[cfg(debug_assertions)]
        return Ok(std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("kal.toml"));

        #[cfg(not(debug_assertions))]
        dirs::home_dir()
            .ok_or(Error::HomeDirNotFound)
            .map(|p| p.join(".config").join("kal.toml"))
    }

    /// Loads config from a toml string
    fn from_toml(toml: &str) -> Result<Self> {
        let span = tracing::debug_span!("config::from_toml");
        let _enter = span.enter();

        toml::from_str(toml).map_err(Into::into)
    }

    /// Loads config from path
    pub fn load_from_path<P: AsRef<Path>>(path: P) -> Result<Self> {
        let path = path.as_ref();

        let span = tracing::debug_span!("config::load_from_path", ?path);
        let _enter = span.enter();

        let toml = std::fs::read_to_string(path)?;
        Self::from_toml(&toml)
    }

    /// Loads config from a canonical path, see [`Self::path`]
    pub fn load() -> Result<Self> {
        let span = tracing::debug_span!("config::load");
        let _enter = span.enter();

        let path = Self::path()?;
        let toml = std::fs::read_to_string(path)?;
        Self::from_toml(&toml)
    }

    /// Loads config from a canonical path, see [`Self::path`]
    pub fn load_with_fallback() -> Self {
        Self::load()
            .inspect_err(|e| tracing::error!("failed to load config, falling back to default: {e}"))
            .unwrap_or_default()
    }

    /// Gets the inner config for specified plugin,
    /// falling back to default if not found or failing to deserialize.
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
}
