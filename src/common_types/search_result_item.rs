use super::Icon;
use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct SearchResultItem {
    pub primary_text: String,
    pub secondary_text: String,
    pub execution_args: Vec<String>,
    pub plugin_name: String,
    pub icon: Icon,
}
