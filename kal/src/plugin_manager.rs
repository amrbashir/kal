use std::sync::RwLock;

use kal_config::Config;
use kal_plugin::ResultItem;

use crate::fuzzyer_matcher::FuzzyMatcher;
use crate::plugin::Plugin;

#[derive(Debug)]
pub struct PluginManager {
    pub plugins: Vec<Plugin>,
    pub max_results: usize,
    pub fuzzy_matcher: RwLock<FuzzyMatcher>,
}

impl PluginManager {
    pub fn new(plugins: Vec<Plugin>) -> Self {
        Self {
            plugins,
            max_results: 0,
            fuzzy_matcher: RwLock::new(FuzzyMatcher::default()),
        }
    }

    pub fn all(config: &Config) -> Self {
        Self::new(vec![
            // kal_plugin_app_launcher::Plugin::new(config).into(),
            // kal_plugin_calculator::Plugin::new(config).into(),
            // kal_plugin_directory_indexer::Plugin::new(config).into(),
            // kal_plugin_everything::Plugin::new(config).into(),
            // kal_plugin_shell::Plugin::new(config).into(),
            // kal_plugin_system_commands::Plugin::new(config).into(),
            // kal_plugin_vscode_workspaces::Plugin::new(config).into(),
            // kal_plugin_workflows::Plugin::new(config).into(),
        ])
    }

    pub fn reload(&mut self, config: &Config) {
        self.max_results = config.general.max_results;

        for plugin in self.plugins.iter_mut() {
            plugin.update_from_config(config);

            // reload plugin reload if enabled
            if plugin.enabled {
                if let Err(e) = plugin.reload(config) {
                    tracing::error!("Failed to reload `{}`: {e}", plugin.name());
                }
            }
        }
    }

    pub fn query(&mut self, query: &str) -> anyhow::Result<Vec<ResultItem>> {
        let mut results = Vec::with_capacity(self.max_results);

        let plugins = &mut self.plugins; // mutability splitting

        // it is fine to block here since only one query can be processed at a time
        let mut matcher = self
            .fuzzy_matcher
            .write()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        // check if a plugin is being invoked directly
        if let Some(plugin) = plugins.iter_mut().find(|p| p.is_direct_invoke(query)) {
            let direct_invoke_len = plugin.direct_invoke_len();
            let new_query = &query[direct_invoke_len..].trim();

            match plugin.query_direct(new_query, &mut matcher) {
                Ok(res) => results.extend(res),
                Err(err) => results.push(plugin.error_item(err.to_string())),
            }
        } else {
            // otherwise, query all queriable plugins
            let trimmed_query = query.trim();

            // queriable plugins are:
            //   1. enabled
            //   2. should be included in global results
            let queriable_plugins = plugins
                .iter_mut()
                .filter(|p| p.enabled && p.include_in_global_results);

            for plugin in queriable_plugins {
                let result = plugin
                    .query(trimmed_query, &mut matcher)
                    .map_err(|e| plugin.error_item(e.to_string()));

                match result {
                    Ok(res) => results.extend(res),
                    Err(r) => results.push(r),
                }
            }
        }

        // sort results by scores in descending order
        results.sort_by(|a, b| b.score.cmp(&a.score));

        Ok(results)
    }
}
