use std::borrow::Cow;

use super::icon::Icon;
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SearchResultItem<'a> {
    pub primary_text: Cow<'a, str>,
    pub secondary_text: Cow<'a, str>,
    pub icon: Icon<'a>,
    pub needs_confirmation: bool,
    pub id: Cow<'a, str>,
    pub score: i64,
}

pub trait IntoSearchResultItem {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<SearchResultItem>;
}
