use std::ops::{Deref, DerefMut};
use std::sync::RwLock;

use kal_config::Config;
use kal_plugin::{Plugin, ResultItem};

pub struct PluginEntry {
    pub enabled: bool,
    pub include_in_global_results: bool,
    pub direct_activation_command: Option<String>,
    plugin: Box<dyn Plugin>,
}

impl std::fmt::Debug for PluginEntry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PluginEntry")
            .field("enabled", &self.enabled)
            .field("include_in_global_results", &self.include_in_global_results)
            .field("direct_activation_command", &self.direct_activation_command)
            .field("plugin_name", &self.plugin.name())
            .finish()
    }
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

    /// Convenient method to construct an error [ResultItem] for this plugin.
    fn error_item(&self, error: String) -> ResultItem {
        ResultItem {
            id: String::new(),
            icon: crate::icon::BuiltinIcon::Error.into(),
            primary_text: self.name().to_owned(),
            secondary_text: error,
            tooltip: None,
            actions: vec![],
            score: 0,
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
pub struct PluginManager {
    pub plugins: Vec<PluginEntry>,
    pub max_results: usize,
    pub fuzzy_matcher: RwLock<kal_plugin::FuzzyMatcher>,
}

impl PluginManager {
    pub fn new(plugins: Vec<PluginEntry>) -> Self {
        Self {
            plugins,
            max_results: 0,
            fuzzy_matcher: RwLock::new(kal_plugin::FuzzyMatcher::default()),
        }
    }

    pub fn all(config: &Config) -> Self {
        Self::new(vec![
            kal_plugin_app_launcher::Plugin::new(config).into(),
            kal_plugin_calculator::Plugin::new(config).into(),
            kal_plugin_directory_indexer::Plugin::new(config).into(),
            kal_plugin_everything::Plugin::new(config).into(),
            kal_plugin_shell::Plugin::new(config).into(),
            kal_plugin_system_commands::Plugin::new(config).into(),
            kal_plugin_vscode_workspaces::Plugin::new(config).into(),
            kal_plugin_workflows::Plugin::new(config).into(),
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
                Ok(res) => res.extend_into(&mut results),
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
                    Ok(r) => r.extend_into(&mut results),
                    Err(r) => results.push(r),
                }
            }
        }

        // sort results by scores in descending order
        results.sort_by(|a, b| b.score.cmp(&a.score));

        Ok(results)
    }
}
