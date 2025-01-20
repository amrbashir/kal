use std::fmt::Display;

use fuzzy_matcher::skim::SkimMatcherV2;
use serde::Serialize;

use crate::icon::{BuiltInIcon, Icon};
use crate::plugin::Plugin;

#[derive(Serialize, Debug)]
pub struct ResultItem {
    pub id: String,
    pub icon: Icon,
    pub primary_text: String,
    pub secondary_text: String,
    pub tooltip: Option<String>,
    pub actions: Vec<Action>,
    pub score: i64,
}

impl ResultItem {
    pub fn plugin_error<S: Display>(plugin: &dyn Plugin, error: S) -> Self {
        Self {
            id: String::new(),
            icon: BuiltInIcon::Error.icon(),
            primary_text: plugin.name().to_owned(),
            secondary_text: error.to_string(),
            tooltip: Some(error.to_string()),
            actions: vec![],
            score: 0,
        }
    }
}

pub trait IntoResultItem {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem>;
}

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

type ActionFn = dyn Fn(&ResultItem) -> anyhow::Result<()>;

#[derive(Serialize)]
pub struct Action {
    pub id: &'static str,
    pub icon: Option<Icon>,
    pub description: Option<&'static str>,
    pub accelerator: Option<&'static str>,
    #[serde(skip)]
    pub action: Box<ActionFn>,
}

impl std::fmt::Debug for Action {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SecondaryAction")
            .field("id", &self.id)
            .field("description", &self.description)
            .field("accelerator", &self.accelerator)
            .field("action", &"<action>")
            .finish()
    }
}

impl Action {
    pub fn new<F>(id: &'static str, action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static,
    {
        Self {
            id,
            icon: None,
            description: None,
            accelerator: None,
            action: Box::new(action),
        }
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    pub fn with_description(mut self, description: &'static str) -> Self {
        self.description = Some(description);
        self
    }

    pub fn with_accelerator(mut self, accelerator: &'static str) -> Self {
        self.accelerator = Some(accelerator);
        self
    }

    pub fn run(&self, item: &ResultItem) -> anyhow::Result<()> {
        (self.action)(item)
    }
}

impl Action {
    pub fn primary<F>(action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static,
    {
        Self {
            id: "RunPrimary",
            description: None,
            icon: None,
            accelerator: Some("Enter"),
            action: Box::new(action),
        }
    }

    pub fn open_elevated<F>(action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static,
    {
        Self::new("RunElevated", action)
            .with_icon(BuiltInIcon::Admin.icon())
            .with_description("Run as adminstrator")
            .with_accelerator("Shift+Enter")
    }

    pub fn open_location<F>(action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static,
    {
        Self::new("OpenLocation", action)
            .with_icon(BuiltInIcon::FolderOpen.icon())
            .with_description("Open containing folder")
            .with_accelerator("Ctrl+O")
    }
}
