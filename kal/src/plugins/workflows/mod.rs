use crate::{
    config::Config,
    icon::{BuiltinIcon, Icon},
    search_result_item::{IntoSearchResultItem, SearchResultItem},
    utils::{self, IteratorExt},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use url::Url;

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", untagged)]
enum WorkflowStep {
    Path {
        path: PathBuf,
    },
    Url {
        url: Url,
    },
    Shell {
        shell: Option<String>,
        script: String,
        working_directory: Option<String>,
        hidden: Option<bool>,
    },
}

#[derive(Serialize, Deserialize, Debug)]
struct Workflow<'a> {
    name: String,
    #[serde(default)]
    id: String,
    description: Option<String>,
    icon: Option<Icon<'a>>,
    #[serde(default)]
    needs_confirmation: bool,
    steps: Vec<WorkflowStep>,
}

impl<'a> Workflow<'a> {
    fn icon(&self) -> Icon<'a> {
        self.icon.clone().unwrap_or(BuiltinIcon::Workflow.icon())
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        for step in &self.steps {
            match &step {
                WorkflowStep::Path { path, .. } => utils::execute(path, elevated)?,
                WorkflowStep::Url { url } => utils::open_url(url)?,
                WorkflowStep::Shell {
                    shell,
                    script,
                    working_directory,
                    hidden,
                } => utils::execute_in_shell(shell, script, working_directory, hidden, elevated)?,
            }
        }

        Ok(())
    }
}

impl<'a> IntoSearchResultItem for Workflow<'a> {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<SearchResultItem> {
        matcher
            .fuzzy_match(&self.name, query)
            .or_else(|| {
                self.description
                    .as_ref()
                    .and_then(|description| matcher.fuzzy_match(description, query))
            })
            .map(|score| SearchResultItem {
                primary_text: self.name.as_str().into(),
                id: self.id.as_str().into(),
                secondary_text: self.description.as_deref().unwrap_or_default().into(),
                icon: self.icon(),
                needs_confirmation: self.needs_confirmation,
                score,
            })
    }
}

#[derive(Debug)]
pub struct Plugin<'a> {
    workflows: Vec<Workflow<'a>>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PluginConfig<'a> {
    #[serde(default)]
    workflows: Vec<Workflow<'a>>,
}

impl<'a> Plugin<'a> {
    const NAME: &'static str = "Workflows";

    fn update_ids(&mut self) {
        for (idx, workflow) in self.workflows.iter_mut().enumerate() {
            workflow.id = if workflow.id.is_empty() {
                format!("{}:{idx}", Self::NAME)
            } else {
                format!("{}:{}", Self::NAME, workflow.id)
            };
        }
    }
}

impl crate::plugin::Plugin for Plugin<'_> {
    fn new(config: &Config, _data_dir: &Path) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Self {
            workflows: config.workflows,
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(self.name());

        self.workflows = config.workflows;
        self.update_ids();

        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
        Ok(self
            .workflows
            .iter()
            .filter_map(|workflow| workflow.fuzzy_match(query, matcher))
            .collect_non_empty())
    }

    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(workflow) = self.workflows.iter().find(|s| s.id == id) {
            workflow.execute(elevated)?;
        }
        Ok(())
    }
}
