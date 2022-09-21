use super::icon::Icon;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SearchResultItem {
    /// The main text to be displayed for this item
    pub primary_text: String,
    /// The secondary text to be displayed for this item
    pub secondary_text: String,
    /// Used when [`crate::plugin::Plugin::execute`] is called
    pub execution_args: serde_json::Value,
    /// The origin of this item
    pub plugin_name: String,
    /// The icon to display next to this item
    pub icon: Icon,
}
