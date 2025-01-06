use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::Context;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::{
    common::SearchResultItem,
    config::{Config, GenericPluginConfig},
};

#[allow(unused_variables)]
pub trait Plugin: Debug {
    fn new(config: &Config, data_dir: &Path) -> anyhow::Result<Self>
    where
        Self: Sized;

    /// Gets the name of the plugin.
    ///
    /// usually used to identify the origin of a [`SearchResultItem`]
    /// and the plugin to exceute it.
    fn name(&self) -> &'static str;

    /// Refreshs the cache and configuration of the plugin
    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    /// Gets [SearchResultItem]s for this query
    fn results(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>>;

    /// Called when `Enter` or `Shift + Enter` are pressed
    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        Ok(())
    }

    /// Called when `CtrlLeft + O` are pressed
    fn reveal_in_dir(&self, id: &str) -> anyhow::Result<()> {
        Ok(())
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

#[derive(Debug)]
pub struct PluginEntry {
    pub enabled: bool,
    pub include_in_global_results: bool,
    pub direct_activation_command: Option<String>,
    plugin: Box<dyn Plugin + Send + 'static>,
}

impl Deref for PluginEntry {
    type Target = dyn Plugin + Send + 'static;

    fn deref(&self) -> &Self::Target {
        self.plugin.as_ref()
    }
}
impl DerefMut for PluginEntry {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.plugin.as_mut()
    }
}

impl PluginEntry {
    fn new<P: Plugin + Send + 'static>(plugin: P) -> Self {
        let config = plugin.default_generic_config();
        Self {
            enabled: config.enabled.unwrap_or(true),
            include_in_global_results: config.include_in_global_results.unwrap_or(true),
            direct_activation_command: config.direct_activation_command,
            plugin: Box::new(plugin),
        }
    }

    pub fn is_directly_invoked(&self, query: &str) -> bool {
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
pub struct PluginStoreInner {
    pub plugins: Vec<PluginEntry>,
}

impl PluginStoreInner {
    pub fn new() -> Self {
        Self {
            plugins: Vec::new(),
        }
    }

    pub fn add<P: Plugin + Send + 'static>(&mut self, plugin: P) {
        self.plugins.push(PluginEntry::new(plugin))
    }

    pub fn find_plugin<F: FnMut(&&mut PluginEntry) -> bool>(
        &mut self,
        f: F,
    ) -> anyhow::Result<&mut PluginEntry> {
        self.plugins
            .iter_mut()
            .find(f)
            .context("Couldn't find plugin")
    }

    pub fn queriable_plugins(&mut self) -> Vec<&mut PluginEntry> {
        self.plugins
            .iter_mut()
            .filter(|p| p.enabled && p.include_in_global_results)
            .collect()
    }

    pub fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
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

            // run plugin refresh if enabled
            if plugin.enabled {
                plugin.refresh(config)?;
            }
        }

        Ok(())
    }

    pub fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        let plugin = self.find_plugin(|p| id.starts_with(p.name()))?;
        plugin.execute(id, elevated)
    }

    pub fn reveal_in_dir(&mut self, id: &str) -> anyhow::Result<()> {
        let plugin = self.find_plugin(|p| id.starts_with(p.name()))?;
        plugin.reveal_in_dir(id)
    }

    pub fn results<'a, 'b>(
        &'a mut self,
        query: &str,
        matcher: &SkimMatcherV2,
        results: &'b mut Vec<SearchResultItem<'a>>,
    ) -> anyhow::Result<()>
    where
        'a: 'b,
    {
        // check if a plugin is being invoked directly
        if let Some(idx) = self
            .plugins
            .iter()
            .position(|p| p.is_directly_invoked(query))
        {
            let invoke_cmd_len = self.plugins[idx].invoke_cmd_len();
            let new_query = &query[invoke_cmd_len..];
            if !new_query.is_empty() {
                if let Ok(Some(res)) = self.plugins[idx].results(new_query, matcher) {
                    results.extend(res);
                }
            }
        } else {
            // otherwise get result from all queriable plugins
            for plugin in self.queriable_plugins() {
                let Ok(Some(res)) = plugin.results(query, matcher) else {
                    continue;
                };
                results.extend(res);
            }
        }

        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct PluginStore(Arc<Mutex<PluginStoreInner>>);

impl PluginStore {
    pub fn new() -> Self {
        Self(Arc::new(Mutex::new(PluginStoreInner::new())))
    }

    pub fn lock(&self) -> MutexGuard<'_, PluginStoreInner> {
        self.0
            .lock()
            .inspect_err(|e| tracing::error!("{e}"))
            .unwrap()
    }

    pub fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        self.lock().refresh(config)
    }

    pub fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        self.lock().execute(id, elevated)
    }

    pub fn reveal_in_dir(&self, id: &str) -> anyhow::Result<()> {
        self.lock().reveal_in_dir(id)
    }
}
