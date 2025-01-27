use fuzzy_matcher::skim::SkimMatcherV2;

use crate::config::{Config, GenericPluginConfig};
use crate::icon::BuiltInIcon;
use crate::result_item::ResultItem;

#[allow(unused_variables)]
#[async_trait::async_trait]
pub trait Plugin: std::fmt::Debug + Send + Sync {
    /// Constructor for plugin.
    fn new(config: &Config) -> Self
    where
        Self: Sized;

    /// Gets the name of the plugin.
    ///
    /// usually used to identify the origin of a [`ResultItem`]
    /// and the plugin to exceute it.
    fn name(&self) -> &'static str;

    /// Default generic config
    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: None,
        }
    }

    /// Convenient method to construct an error [ResultItem] for this plugin.
    fn error_item(&self, error: String) -> ResultItem {
        ResultItem {
            id: String::new(),
            icon: BuiltInIcon::Error.into(),
            primary_text: self.name().to_owned(),
            secondary_text: error,
            tooltip: None,
            actions: vec![],
            score: 0,
        }
    }

    /// Reloads the cache and configuration of the plugin
    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    /// Query the plugin for [`ResultItem`]s.
    async fn query(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<PluginQueryOutput> {
        Ok(PluginQueryOutput::None)
    }

    /// Query the plugin for [`ResultItem`]s when directly invoked.
    async fn query_direct(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<PluginQueryOutput> {
        self.query(query, matcher).await
    }
}

/// Possible output from querying a plugin.
pub enum PluginQueryOutput {
    None,
    One(ResultItem),
    Multiple(Vec<ResultItem>),
}

impl PluginQueryOutput {
    pub fn extend_into(self, results: &mut Vec<ResultItem>) {
        match self {
            PluginQueryOutput::None => {}
            PluginQueryOutput::One(one) => results.push(one),
            PluginQueryOutput::Multiple(multiple) => results.extend(multiple),
        }
    }
}

impl From<ResultItem> for PluginQueryOutput {
    fn from(value: ResultItem) -> Self {
        PluginQueryOutput::One(value)
    }
}

impl From<Vec<ResultItem>> for PluginQueryOutput {
    fn from(value: Vec<ResultItem>) -> Self {
        PluginQueryOutput::Multiple(value)
    }
}

impl From<Option<ResultItem>> for PluginQueryOutput {
    fn from(value: Option<ResultItem>) -> Self {
        match value {
            Some(value) => PluginQueryOutput::One(value),
            None => PluginQueryOutput::None,
        }
    }
}

impl From<Option<Vec<ResultItem>>> for PluginQueryOutput {
    fn from(value: Option<Vec<ResultItem>>) -> Self {
        match value {
            Some(value) => PluginQueryOutput::Multiple(value),
            None => PluginQueryOutput::None,
        }
    }
}
