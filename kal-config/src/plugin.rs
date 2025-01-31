use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct PluginConfig {
    /// Whether this plugin is enabled or not.
    pub enabled: Option<bool>,
    /// Whether to include this plugin in results in global queries.
    pub include_in_global_results: Option<bool>,
    /// Direct activation command for this plugin.
    pub direct_activation_command: Option<String>,

    /// An opaque type represnting plugin config options.
    #[serde(flatten)]
    pub inner: Option<toml::Table>,
}

impl PluginConfig {
    /// Applies default options from `another`.
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

    /// Whether this plugin is enabled or not.
    ///
    /// Default: `true`
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    /// Whether to include this plugin in results in global queries.
    ///
    /// Default: `true`
    pub fn include_in_global_results(&self) -> bool {
        self.include_in_global_results.unwrap_or(true)
    }
}
