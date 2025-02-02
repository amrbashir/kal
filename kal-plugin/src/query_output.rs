use crate::ResultItem;

/// Possible output from querying a plugin.
pub enum PluginQueryOutput {
    None,
    One(ResultItem),
    Multiple(Vec<ResultItem>),
}

impl PluginQueryOutput {
    pub fn extend_into(self, results: &mut Vec<ResultItem>) {
        match self {
            PluginQueryOutput::None => {}
            PluginQueryOutput::One(one) => results.push(one),
            PluginQueryOutput::Multiple(multiple) => results.extend(multiple),
        }
    }
}

impl From<ResultItem> for PluginQueryOutput {
    fn from(value: ResultItem) -> Self {
        PluginQueryOutput::One(value)
    }
}

impl From<Vec<ResultItem>> for PluginQueryOutput {
    fn from(value: Vec<ResultItem>) -> Self {
        PluginQueryOutput::Multiple(value)
    }
}

impl From<Option<ResultItem>> for PluginQueryOutput {
    fn from(value: Option<ResultItem>) -> Self {
        match value {
            Some(value) => PluginQueryOutput::One(value),
            None => PluginQueryOutput::None,
        }
    }
}

impl From<Option<Vec<ResultItem>>> for PluginQueryOutput {
    fn from(value: Option<Vec<ResultItem>>) -> Self {
        match value {
            Some(value) => PluginQueryOutput::Multiple(value),
            None => PluginQueryOutput::None,
        }
    }
}
