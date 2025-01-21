use std::fmt::Debug;
use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;

use crate::config::{Config, GenericPluginConfig};
use crate::result_item::ResultItem;

#[allow(unused_variables)]
pub trait Plugin: Debug {
    /// Constructor for plugin.
    fn new(config: &Config, data_dir: &Path) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Gets the name of the plugin.
    ///
    /// usually used to identify the origin of a [`ResultItem`]
    /// and the plugin to exceute it.
    fn name(&self) -> &'static str;

    /// Reloads the cache and configuration of the plugin
    fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    /// Query the plugin for [`ResultItem`]s.
    fn query(&mut self, query: &str, matcher: &SkimMatcherV2) -> anyhow::Result<PluginQueryOutput>;

    /// Query the plugin for [`ResultItem`]s when directly invoked.
    fn query_direct(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<PluginQueryOutput> {
        self.query(query, matcher)
    }

    /// Default generic config
    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: None,
        }
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
