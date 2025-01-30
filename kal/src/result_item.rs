use serde::Serialize;

use crate::icon::{BuiltinIcon, Icon};

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
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut crate::fuzzy_matcher::Matcher,
    ) -> Option<ResultItem>;
}

type ActionFn = dyn Fn(&ResultItem) -> anyhow::Result<()> + Send + Sync;

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
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
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
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
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
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
    {
        Self::new("RunElevated", action)
            .with_icon(BuiltinIcon::Admin.into())
            .with_description("Run as adminstrator")
            .with_accelerator("Shift+Enter")
    }

    pub fn open_location<F>(action: F) -> Self
    where
        F: Fn(&ResultItem) -> anyhow::Result<()> + 'static + Send + Sync,
    {
        Self::new("OpenLocation", action)
            .with_icon(BuiltinIcon::FolderOpen.into())
            .with_description("Open containing folder")
            .with_accelerator("Ctrl+O")
    }
}
