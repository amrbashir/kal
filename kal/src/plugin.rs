use std::fmt::Debug;
use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;

use crate::config::{Config, GenericPluginConfig};
use crate::result_item::QueryReturn;

#[allow(unused_variables)]
pub trait Plugin: Debug {
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

    /// Gets [ResultItem]s for this query from the plugin.
    fn query(&mut self, query: &str, matcher: &SkimMatcherV2) -> anyhow::Result<QueryReturn>;

    /// Gets [ResultItem]s for this query from the plugin when being directly invoked.
    fn query_direct(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<QueryReturn> {
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
