use crate::common_types::SearchResultItem;

pub trait Plugin {
    fn name(&self) -> &str;
    fn refresh(&mut self);
    fn results(&self, query: &str) -> &[SearchResultItem];
    fn execute(&self, item: &SearchResultItem, elevated: bool);
    fn open_location(&self, item: &SearchResultItem);
}
