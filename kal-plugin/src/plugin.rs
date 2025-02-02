use kal_config::{Config, PluginConfig};

use crate::{FuzzyMatcher, PluginQueryOutput};

#[async_trait::async_trait]
pub trait Plugin: Send + Sync {
    /// Constructor for plugin.
    fn new(config: &Config) -> Self
    where
        Self: Sized;

    /// Gets the name of the plugin.
    ///
    /// usually used to identify the origin of a [`ResultItem`]
    /// and the plugin to exceute it.
    fn name(&self) -> &'static str;

    /// Default plugin config
    fn default_plugin_config(&self) -> PluginConfig {
        PluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: None,
            inner: None,
        }
    }

    /// Reloads the cache and configuration of the plugin
    #[allow(unused_variables)]
    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    /// Query the plugin for [`ResultItem`]s.
    #[allow(unused_variables)]
    async fn query(
        &mut self,
        query: &str,
        matcher: &mut FuzzyMatcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        Ok(PluginQueryOutput::None)
    }

    /// Query the plugin for [`ResultItem`]s when directly invoked.
    async fn query_direct(
        &mut self,
        query: &str,
        matcher: &mut crate::fuzzy_matcher::FuzzyMatcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        self.query(query, matcher).await
    }
}
