use std::borrow::Cow;

use super::icon::Icon;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SearchResultItem<'a> {
    pub primary_text: Cow<'a, str>,
    pub secondary_text: Cow<'a, str>,
    pub icon: Icon<'a>,
    pub needs_confirmation: bool,
    pub identifier: Cow<'a, str>,
}
