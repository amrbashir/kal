use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::Context;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Deserialize;

use crate::{common::SearchResultItem, config::Config};

pub trait Plugin: Debug {
    fn new(config: &Config) -> anyhow::Result<Self>
    where
        Self: Sized;
    /// Gets the name of the plugin.
    ///
    /// usually used to identify the origin of a [`SearchResultItem`]
    /// and the plugin to exceute it.
    fn name(&self) -> &str;
    /// Refreshs the cache and configuration of the plugin
    fn refresh(&mut self, config: &Config) -> anyhow::Result<()>;
    /// Gets [SearchResultItem]s for this query
    fn results(
        &self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Vec<SearchResultItem<'_>>>;
    /// Called when `Enter` or `Shift + Enter` are pressed
    fn execute(&self, identifier: &str, elevated: bool) -> anyhow::Result<()>;
    /// Called when `CtrlLeft + O` are pressed
    fn reveal_in_dir(&self, #[allow(unused)] identifier: &str) -> anyhow::Result<()> {
        Ok(())
    }
}

#[derive(Deserialize)]
struct BasePluginConfig {
    enabled: bool,
}

impl Default for BasePluginConfig {
    fn default() -> Self {
        Self { enabled: true }
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

    pub fn find_plugin<F: FnMut(&&PluginEntry) -> bool>(
        &self,
        f: F,
    ) -> anyhow::Result<&PluginEntry> {
        self.plugins.iter().find(f).context("Couldn't find plugin")
    }

    pub fn plugins(&self) -> Vec<&PluginEntry> {
        self.plugins.iter().filter(|p| p.enabled).collect()
    }

    pub fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        for plugin in self.plugins.iter_mut() {
            // update plugin enabled status
            let c = config.plugin_config::<BasePluginConfig>(plugin.name());
            plugin.enabled = c.enabled;

            // run plugin refresh if enabled
            if plugin.enabled {
                plugin.refresh(config)?;
            }
        }

        Ok(())
    }

    pub fn execute(&self, id: &str, elevated: bool) -> anyhow::Result<()> {
        let plugin = self.find_plugin(|p| id.starts_with(p.name()))?;
        plugin.execute(id, elevated)
    }

    pub fn reveal_in_dir(&self, id: &str) -> anyhow::Result<()> {
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

    pub fn execute(&self, id: &str, elevated: bool) -> anyhow::Result<()> {
        self.lock().execute(id, elevated)
    }

    pub fn reveal_in_dir(&self, id: &str) -> anyhow::Result<()> {
        self.lock().reveal_in_dir(id)
    }
}
