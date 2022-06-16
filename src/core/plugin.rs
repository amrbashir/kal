use crate::common::SearchResultItem;

pub trait Plugin {
    /// Gets the name of the plugin.
    ///
    /// usually used to identify the origin of a [`SearchResultItem`]
    /// and the plugin to exceute it.
    fn name(&self) -> &str;
    /// Gets whether a plugin is enabled or not
    fn enabled(&self) -> bool;
    /// Refreshs the cache and configuration of the plugin
    fn refresh(&mut self);
    /// Gets [SearchResultItem]s from the plugin for this query
    fn results(&self, query: &str) -> &[SearchResultItem];
    /// Called when `Enter` or `Shift + Enter` are pressed
    fn execute(&self, item: &SearchResultItem, elevated: bool);
    /// Called when `CtrlLeft + O` are pressed
    fn open_location(&self, item: &SearchResultItem);
}

/// The plugin struct must have methods with the same signature as the methods from the [`Plugin`] trait.
macro_rules! impl_plugin {
    ($plugin:ident) => {
        impl $crate::plugin::Plugin for $plugin {
            fn name(&self) -> &str {
                self.name()
            }

            fn enabled(&self) -> bool {
                self.enabled()
            }

            fn refresh(&mut self) {
                self.refresh()
            }

            fn results(&self, query: &str) -> &[SearchResultItem] {
                self.results(query)
            }

            fn execute(&self, item: &SearchResultItem, elevated: bool) {
                self.execute(item, elevated)
            }

            fn open_location(&self, item: &SearchResultItem) {
                self.open_location(item);
            }
        }
    };
}

pub(crate) use impl_plugin;
