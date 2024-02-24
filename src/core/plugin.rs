use std::fmt::Debug;

use crate::{common::SearchResultItem, config::Config};

pub trait Plugin: Debug {
    fn new(config: &Config) -> anyhow::Result<Box<Self>>
    where
        Self: Sized;
    /// Gets the name of the plugin.
    ///
    /// usually used to identify the origin of a [`SearchResultItem`]
    /// and the plugin to exceute it.
    fn name(&self) -> &str;
    /// Gets whether a plugin is enabled or not
    fn enabled(&self) -> bool;
    /// Refreshs the cache and configuration of the plugin
    fn refresh(&mut self, config: &Config) -> anyhow::Result<()>;
    /// Gets [SearchResultItem]s for this query
    fn results(&self, query: &str) -> anyhow::Result<&[SearchResultItem]>;
    /// Called when `Enter` or `Shift + Enter` are pressed
    fn execute(&self, item: &SearchResultItem, elevated: bool) -> anyhow::Result<()>;
    /// Called when `CtrlLeft + O` are pressed
    fn open_location(&self, #[allow(unused)] item: &SearchResultItem) -> anyhow::Result<()> {
        Ok(())
    }
}
