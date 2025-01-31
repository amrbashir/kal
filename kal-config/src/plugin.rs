use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PluginConfig {
    /// Whether this plugin is enabled or not.
    pub enabled: Option<bool>,
    /// Whether to include this plugin in results in global queries.
    #[serde(
        alias = "include_in_global_results",
        alias = "include-in-global-results"
    )]
    pub include_in_global_results: Option<bool>,
    /// Direct activation command for this plugin.
    #[serde(
        alias = "direct_activation_command",
        alias = "direct-activation-command"
    )]
    pub direct_activation_command: Option<String>,

    /// An opaque type represnting plugin config options.
    #[serde(flatten)]
    pub inner: Option<toml::Table>,
}

impl JsonSchema for PluginConfig {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "PluginConfig".into()
    }

    fn json_schema(_generator: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema! ({
            "type": ["object", "null"],
            "properties": {
                "enabled": {
                    "type": ["boolean", "null"],
                    "description": "Whether this plugin is enabled or not."
                },
                "includeInGlobalResults": {
                    "type": ["boolean", "null"],
                    "description": "Whether to include this plugin in results in global queries."
                },
                "directActivationCommand": {
                    "type": ["string", "null"],
                    "description": "Direct activation command for this plugin."
                },
            }
        })
    }
}

impl PluginConfig {
    /// Whether this plugin is enabled or not.
    ///
    /// Default: `true`
    pub fn enabled(&self) -> bool {
        self.enabled.unwrap_or(true)
    }

    /// Whether this plugin is enabled or not.
    /// Falling back to provided default if `Some`.
    ///
    /// Default: `true`
    pub fn enabled_or(&self, enabled: Option<bool>) -> bool {
        self.enabled.or(enabled).unwrap_or(true)
    }

    /// Whether to include this plugin in results in global queries.
    ///
    /// Default: `true`
    pub fn include_in_global_results(&self) -> bool {
        self.include_in_global_results.unwrap_or(true)
    }

    /// Whether to include this plugin in results in global queries.
    ///
    /// Falling back to provided default if `Some`.
    ///
    /// Default: `true`
    pub fn include_in_global_results_or(&self, include: Option<bool>) -> bool {
        self.include_in_global_results.or(include).unwrap_or(true)
    }

    /// Direct activation command for this plugin.
    pub fn direct_activation_command(&self) -> Option<String> {
        self.direct_activation_command.clone()
    }

    /// Direct activation command for this plugin.
    ///
    /// Falling back to provided default if `Some`.
    ///
    /// Default: `true`
    pub fn direct_activation_command_or(&self, include: Option<&String>) -> Option<String> {
        self.direct_activation_command
            .clone()
            .or_else(|| include.cloned())
    }
}
