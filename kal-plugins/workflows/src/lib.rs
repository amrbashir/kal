use std::path::PathBuf;

use kal_config::Config;
use kal_plugin::{Action, BuiltinIcon, Icon, IntoResultItem, PluginQueryOutput, ResultItem};
use kal_utils::{IteratorExt, PathExt};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug)]
pub struct Plugin {
    workflows: Vec<Workflow>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
struct PluginConfig {
    #[serde(default)]
    workflows: Vec<Workflow>,
}

impl Plugin {
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

    fn all(&self) -> Option<Vec<ResultItem>> {
        self.workflows
            .iter()
            .map(|workflow| workflow.item(0))
            .collect_non_empty()
    }

    fn all_for_query(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<Vec<ResultItem>> {
        self.workflows
            .iter()
            .filter_map(|workflow| workflow.fuzzy_match(query, matcher))
            .collect_non_empty()
    }
}

#[async_trait::async_trait]
impl kal_plugin::Plugin for Plugin {
    fn new(config: &Config) -> Self {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        let mut plugin = Self {
            workflows: config.workflows,
        };

        plugin.update_ids();

        plugin
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_plugin_config(&self) -> kal_config::PluginConfig {
        kal_config::PluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: Some("@".into()),
            inner: None,
        }
    }

    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(self.name());

        self.workflows = config.workflows;
        self.update_ids();

        Ok(())
    }

    async fn query(
        &mut self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        Ok(self.all_for_query(query, matcher).into())
    }

    async fn query_direct(
        &mut self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        if query.is_empty() {
            Ok(self.all().into())
        } else {
            Ok(self.all_for_query(query, matcher).into())
        }
    }
}

#[derive(Clone, Deserialize, Serialize, Debug, PartialEq, Eq)]
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

#[derive(Clone, Debug, Serialize, Deserialize)]
struct Workflow {
    name: String,
    #[serde(default)]
    id: String,
    description: Option<String>,
    icon: Option<Icon>,
    #[serde(default)]
    needs_confirmation: bool,
    steps: Vec<WorkflowStep>,
}

impl Workflow {
    fn icon(&self) -> Icon {
        self.icon.clone().unwrap_or(BuiltinIcon::Workflow.into())
    }

    fn confirmed(&self) -> bool {
        if !self.needs_confirmation {
            return true;
        }

        let res = rfd::MessageDialog::new()
            .set_title("Please confirm")
            .set_description(format!("You are about to run {}, are you sure?", self.name))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();

        res == rfd::MessageDialogResult::Yes
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        if !self.confirmed() {
            return Ok(());
        }

        for step in &self.steps {
            match step {
                WorkflowStep::Path { path, .. } => {
                    let path = path.replace_env();
                    kal_utils::execute(path, elevated)?
                }
                WorkflowStep::Url { url } => kal_utils::open_url(url)?,
                WorkflowStep::Shell {
                    shell,
                    script,
                    working_directory,
                    hidden,
                } => kal_utils::execute_in_shell(
                    shell.as_ref(),
                    script,
                    working_directory.as_ref(),
                    *hidden,
                    elevated,
                )?,
            }
        }

        Ok(())
    }

    fn item(&self, score: u16) -> ResultItem {
        let workflow = self.clone();
        let open = Action::primary(move |_| workflow.execute(false));

        let workflow = self.clone();
        let open_elevated = Action::open_elevated(move |_| workflow.execute(true));

        ResultItem {
            id: self.id.as_str().into(),
            icon: self.icon(),
            primary_text: self.name.as_str().into(),
            secondary_text: self.description.as_deref().unwrap_or("Workflow").into(),
            tooltip: None,
            actions: vec![open, open_elevated],
            score,
        }
    }
}

impl IntoResultItem for Workflow {
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<ResultItem> {
        matcher
            .fuzzy_match(&self.name, query)
            .or_else(|| {
                self.description
                    .as_ref()
                    .and_then(|description| matcher.fuzzy_match(description, query))
            })
            .map(|score| self.item(score))
    }
}
