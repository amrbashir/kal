use std::ops::{Deref, DerefMut};

use fuzzy_matcher::skim::SkimMatcherV2;
use smol::prelude::*;

use crate::config::Config;
use crate::plugin::Plugin;
use crate::result_item::ResultItem;

#[derive(Debug)]
pub struct PluginEntry {
    pub enabled: bool,
    pub include_in_global_results: bool,
    pub direct_activation_command: Option<String>,
    plugin: Box<dyn Plugin>,
}

impl Deref for PluginEntry {
    type Target = dyn Plugin;

    fn deref(&self) -> &Self::Target {
        self.plugin.as_ref()
    }
}
impl DerefMut for PluginEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.plugin.as_mut()
    }
}

impl<P: Plugin + 'static> From<P> for PluginEntry {
    fn from(value: P) -> Self {
        Self::new(value)
    }
}

impl PluginEntry {
    fn new<P: Plugin + 'static>(plugin: P) -> Self {
        let config = plugin.default_generic_config();
        Self {
            enabled: config.enabled.unwrap_or(true),
            include_in_global_results: config.include_in_global_results.unwrap_or(true),
            direct_activation_command: config.direct_activation_command,
            plugin: Box::new(plugin),
        }
    }

    pub fn is_direct_invoke(&self, query: &str) -> bool {
        self.direct_activation_command
            .as_deref()
            .map(|c| query.starts_with(c))
            .unwrap_or(false)
    }

    pub fn invoke_cmd_len(&self) -> usize {
        self.direct_activation_command
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_default()
    }
}

#[derive(Debug)]
pub struct PluginStore {
    pub plugins: Vec<PluginEntry>,
}

impl PluginStore {
    pub fn new(plugins: Vec<PluginEntry>) -> Self {
        Self { plugins }
    }

    pub async fn reload(&mut self, config: &Config) {
        for plugin in self.plugins.iter_mut() {
            // update plugin generic config
            let default_generic_config = plugin.default_generic_config();
            let generic_config = config
                .generic_config(plugin.name())
                .map(|c| c.apply_from(&default_generic_config))
                .unwrap_or_else(|| default_generic_config);

            plugin.enabled = generic_config.enabled();
            plugin.include_in_global_results = generic_config.include_in_global_results();
            plugin.direct_activation_command = generic_config.direct_activation_command;

            // run plugin reload if enabled
            if plugin.enabled {
                if let Err(e) = plugin.reload(config).await {
                    tracing::error!("Failed to reload `{}`: {e}", plugin.name());
                }
            }
        }
    }

    pub fn queriable_plugins(&mut self) -> impl Iterator<Item = &mut PluginEntry> {
        self.plugins
            .iter_mut()
            .filter(|p| p.enabled && p.include_in_global_results)
    }

    pub async fn query(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
        results: &mut Vec<ResultItem>,
    ) -> anyhow::Result<()> {
        // check if a plugin is being invoked directly
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.is_direct_invoke(query)) {
            let invoke_cmd_len = plugin.invoke_cmd_len();
            let new_query = &query[invoke_cmd_len..].trim();

            match plugin.query_direct(new_query, matcher).await {
                Ok(res) => res.extend_into(results),
                Err(err) => results.push(plugin.error_item(err.to_string())),
            }
        } else {
            let trimmed_query = query.trim();

            // otherwise get result from all queriable plugins
            let mut results_iter = smol::stream::iter(self.queriable_plugins()).map(|p| async {
                p.query(trimmed_query, matcher)
                    .await
                    .map_err(|e| p.error_item(e.to_string()))
            });

            while let Some(r) = results_iter.next().await {
                match r.await {
                    Ok(r) => r.extend_into(results),
                    Err(r) => results.push(r),
                }
            }
        }

        Ok(())
    }
}
