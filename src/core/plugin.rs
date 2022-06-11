use crate::common_types::SearchResultItem;

pub trait Plugin {
    fn name(&self) -> &str;
    fn refresh(&mut self);
    fn results(&self, query: &str) -> &[SearchResultItem];
    fn execute(&self, item: &SearchResultItem, elevated: bool);
    fn open_location(&self, item: &SearchResultItem);
}

/// The plugin struct must have methods with the same signature as the methods from the [`Plugin`] trait.
macro_rules! impl_plugin {
    ($plugin:ident) => {
        impl Plugin for $plugin {
            fn name(&self) -> &str {
                self.name()
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
