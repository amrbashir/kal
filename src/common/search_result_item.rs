use std::path::PathBuf;

use super::icon::Icon;
use anyhow::Context;
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
    /// Whether execution of this item, requires confirmation or not
    pub needs_confirmation: bool,
}

impl SearchResultItem {
    pub fn str(&self) -> anyhow::Result<&str> {
        self.execution_args
            .as_str()
            .with_context(|| "JSON value not a str")
    }

    pub fn path(&self) -> anyhow::Result<PathBuf> {
        self.str().map(PathBuf::from)
    }

    pub fn index(&self) -> anyhow::Result<u64> {
        let index = self
            .execution_args
            .as_u64()
            .with_context(|| "JSON value not u64")?;
        Ok(index)
    }
}
