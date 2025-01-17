use std::borrow::Cow;

use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Serialize;

use crate::icon::Icon;

#[derive(Serialize, Debug, Clone)]
pub struct ResultItem<'a> {
    pub id: Cow<'a, str>,
    pub score: i64,
    pub primary_text: Cow<'a, str>,
    pub secondary_text: Cow<'a, str>,
    pub icon: Icon<'a>,
    pub needs_confirmation: bool,
}

pub trait IntoResultItem {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem>;
}
