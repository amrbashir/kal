use crate::common_types::SearchResultItem;
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};

pub fn fuzzy_sort(query: &str, items: Vec<SearchResultItem>) -> Vec<SearchResultItem> {
    let matcher = SkimMatcherV2::default();
    let mut sorted_items = Vec::new();
    for item in items {
        let score = matcher
            .fuzzy_match(&item.primary_text, query)
            .unwrap_or_default();

        sorted_items.push((score, item));
    }

    sorted_items.sort_by_key(|i| i.0);
    sorted_items.into_iter().rev().map(|i| i.1).collect()
}
