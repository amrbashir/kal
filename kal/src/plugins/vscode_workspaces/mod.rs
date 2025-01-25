use std::path::PathBuf;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::Deserialize;
use sqlite::OpenFlags;

use crate::config::{Config, GenericPluginConfig};
use crate::icon::BuiltInIcon;
use crate::plugin::PluginQueryOutput;
use crate::result_item::{Action, IntoResultItem, ResultItem};
use crate::utils::{self, IteratorExt};

#[derive(Debug)]
pub struct Plugin {
    workspaces: Vec<Workspace>,
}

impl Plugin {
    const NAME: &str = "VSCodeWorkspaces";
    const WORKSPACES_QUERY: &str =
        "SELECT value FROM ItemTable WHERE key LIKE 'history.recentlyOpenedPathsList'";
}

#[async_trait::async_trait]
impl crate::plugin::Plugin for Plugin {
    fn new(_config: &crate::config::Config, _data_dir: &std::path::Path) -> Self {
        Self {
            workspaces: Vec::new(),
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(false),
            direct_activation_command: Some("{".into()),
        }
    }

    async fn reload(&mut self, _config: &Config) -> anyhow::Result<()> {
        let Some(roaming) = dirs::data_dir() else {
            return Ok(());
        };

        let vscode_appdata = roaming.join("Code");
        let vscdb = vscode_appdata.join("User/globalStorage/state.vscdb");

        let flags = OpenFlags::new().with_read_only();
        let conn = sqlite::Connection::open_with_flags(vscdb, flags)?;

        let mut stmt = conn.prepare(Self::WORKSPACES_QUERY)?;
        let _ = stmt.next()?;
        let json = stmt.read::<String, _>(0)?;

        let workspaces = serde_json::from_str::<WorkspacesJson>(&json)?;

        self.workspaces = workspaces
            .entries
            .into_iter()
            .filter(|w| w.folder_uri.is_some())
            .map(|w| Workspace::new(w.folder_uri.unwrap()))
            .collect();

        Ok(())
    }

    async fn query_direct(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<PluginQueryOutput> {
        Ok(self
            .workspaces
            .iter()
            .filter_map(|w| w.fuzzy_match(query, matcher))
            .collect_non_empty::<Vec<_>>()
            .into())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct WorkspacesJsonEntry {
    folder_uri: Option<url::Url>,
}

#[derive(Debug, Deserialize)]
struct WorkspacesJson {
    entries: Vec<WorkspacesJsonEntry>,
}

#[derive(Debug)]
struct Workspace {
    name: String,
    path: PathBuf,
    uri: url::Url,
}

impl Workspace {
    fn new(uri: url::Url) -> Self {
        let name = uri.path().split('/').last().unwrap_or_default();
        let mut name = name.to_owned();

        let authority = uri.authority();
        if !authority.is_empty() {
            let (remote, machine) = authority.split_once("%2B").unwrap_or_default();
            name = format!("{name} - {machine} ({})", remote.to_uppercase());
        };

        let path = uri.to_file_path().unwrap_or_else(|_| {
            let path = uri.path();
            let path = percent_encoding::percent_decode_str(path).decode_utf8_lossy();
            let path = PathBuf::from(path.as_ref());
            path.canonicalize().unwrap_or(path)
        });

        Self { uri, name, path }
    }
}

impl Workspace {
    fn item(&self, score: i64) -> ResultItem {
        let tooltip = format!("{}\n{}", self.name, self.path.display());
        let uri = self.uri.clone();

        ResultItem {
            id: format!("{}:{}", Plugin::NAME, self.name),
            icon: BuiltInIcon::Code.into(),
            primary_text: self.name.clone(),
            secondary_text: self.path.to_string_lossy().to_string(),
            tooltip: Some(tooltip),
            actions: vec![Action::primary(move |_| {
                utils::execute_with_args(
                    "code",
                    format!("--folder-uri {}", uri.as_str()),
                    false,
                    true,
                )
            })],
            score,
        }
    }
}

impl IntoResultItem for Workspace {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem> {
        matcher
            .fuzzy_match(&self.name, query)
            .map(|score| self.item(score))
    }
}
