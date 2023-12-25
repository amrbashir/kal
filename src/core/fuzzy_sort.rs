use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

use crate::common::SearchResultItem;

pub trait CanBeFuzzed {
    fn key(&self) -> &str;
}

impl<T: AsRef<str>> CanBeFuzzed for T {
    fn key(&self) -> &str {
        self.as_ref()
    }
}

impl CanBeFuzzed for SearchResultItem {
    fn key(&self) -> &str {
        &self.primary_text
    }
}

/// Sorts a vector of structs by field
pub fn fuzzy_sort<T: CanBeFuzzed>(matcher: &SkimMatcherV2, items: &mut [T], query: &str) {
    items.sort_by_cached_key(|item| matcher.fuzzy_match(item.key(), query));
    items.reverse();
}
