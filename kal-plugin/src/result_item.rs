use serde::Serialize;

use crate::{Action, Icon};

#[derive(Serialize, Debug)]
pub struct ResultItem {
    pub id: String,
    pub icon: Icon,
    pub primary_text: String,
    pub secondary_text: String,
    pub tooltip: Option<String>,
    pub actions: Vec<Action>,
    pub score: u16,
}

pub trait IntoResultItem {
    fn fuzzy_match(&self, query: &str, matcher: &mut crate::FuzzyMatcher) -> Option<ResultItem>;
}
