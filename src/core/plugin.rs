use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::Context;
use fuzzy_matcher::skim::SkimMatcherV2;

use crate::{common::SearchResultItem, config::Config};

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
    fn execute(&mut self, identifier: &str, elevated: bool) -> anyhow::Result<()> {
        Ok(())
    }
    /// Called when `CtrlLeft + O` are pressed
    fn reveal_in_dir(&self, identifier: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Debug)]
pub struct PluginEntry {
    enabled: bool,
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
        Self {
            enabled: true,
            plugin: Box::new(plugin),
        }
    }
}

#[derive(Debug)]
pub struct PluginStoreInner {
    plugins: Vec<PluginEntry>,
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

    pub fn plugins(&mut self) -> Vec<&mut PluginEntry> {
        self.plugins.iter_mut().filter(|p| p.enabled).collect()
    }

    pub fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        for plugin in self.plugins.iter_mut() {
            // update plugin enabled status
            plugin.enabled = config.is_plugin_enabled(plugin.name());

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
