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

pub trait FuzzySort {
    fn fuzzy_sort(&mut self, query: &str, matcher: &SkimMatcherV2);
}

impl<T: CanBeFuzzed> FuzzySort for [T] {
    fn fuzzy_sort(&mut self, query: &str, matcher: &SkimMatcherV2) {
        fuzzy_sort(self, query, matcher)
    }
}

/// Sorts a vector of structs by field
pub fn fuzzy_sort<T: CanBeFuzzed>(items: &mut [T], query: &str, matcher: &SkimMatcherV2) {
    items.sort_by_cached_key(|item| matcher.fuzzy_match(item.key(), query));
    items.reverse();
}
