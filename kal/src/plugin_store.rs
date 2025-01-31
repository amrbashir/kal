use std::ops::{Deref, DerefMut};

use kal_config::Config;

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
        let config = plugin.default_plugin_config();
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

    pub fn direct_invoke_len(&self) -> usize {
        self.direct_activation_command
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_default()
    }

    fn update_from_config(&mut self, config: &Config) {
        let default_c = self.default_plugin_config();

        match config.plugins.get(self.name()) {
            Some(c) => {
                self.enabled = c.enabled_or(default_c.enabled);
                self.include_in_global_results =
                    c.include_in_global_results_or(default_c.include_in_global_results);
                self.direct_activation_command =
                    c.direct_activation_command_or(default_c.direct_activation_command.as_ref());
            }
            None => {
                self.enabled = default_c.enabled();
                self.include_in_global_results = default_c.include_in_global_results();
                self.direct_activation_command = default_c.direct_activation_command();
            }
        };
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
            plugin.update_from_config(config);

            // reload plugin reload if enabled
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
        matcher: &mut crate::fuzzy_matcher::Matcher,
        results: &mut Vec<ResultItem>,
    ) -> anyhow::Result<()> {
        // check if a plugin is being invoked directly
        if let Some(plugin) = self.plugins.iter_mut().find(|p| p.is_direct_invoke(query)) {
            let direct_invoke_len = plugin.direct_invoke_len();
            let new_query = &query[direct_invoke_len..].trim();

            match plugin.query_direct(new_query, matcher).await {
                Ok(res) => res.extend_into(results),
                Err(err) => results.push(plugin.error_item(err.to_string())),
            }
        } else {
            let trimmed_query = query.trim();

            for plugin in self.queriable_plugins() {
                let result = plugin
                    .query(trimmed_query, matcher)
                    .await
                    .map_err(|e| plugin.error_item(e.to_string()));

                match result {
                    Ok(r) => r.extend_into(results),
                    Err(r) => results.push(r),
                }
            }
        }

        Ok(())
    }
}
