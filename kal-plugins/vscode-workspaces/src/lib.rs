use std::path::PathBuf;

use kal_config::Config;
use kal_plugin::{Action, BuiltinIcon, Icon, IntoResultItem, PluginQueryOutput, ResultItem};
use kal_utils::IteratorExt;
use serde::Deserialize;
use sqlite::OpenFlags;

#[derive(Debug)]
pub struct Plugin {
    workspaces: Vec<Workspace>,
}

impl Plugin {
    const NAME: &str = "VSCodeWorkspaces";
    const WORKSPACES_QUERY: &str =
        "SELECT value FROM ItemTable WHERE key LIKE 'history.recentlyOpenedPathsList'";
}


impl kal_plugin::Plugin for Plugin {
    fn new(_config: &Config) -> Self {
        Self {
            workspaces: Vec::new(),
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_plugin_config(&self) -> kal_config::PluginConfig {
        kal_config::PluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(false),
            direct_activation_command: Some("{".into()),
            inner: None,
        }
    }

    fn reload(&mut self, _config: &Config) -> anyhow::Result<()> {
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

        let icon = match dirs::data_local_dir() {
            Some(localappdata) => {
                let folder_icon = &vscode_appdata;
                let vscode_icon = localappdata.join("Programs/Microsoft VS Code/Code.exe");
                Icon::overlay(folder_icon.to_string_lossy(), vscode_icon.to_string_lossy())
            }
            None => BuiltinIcon::Code.into(),
        };

        self.workspaces = workspaces
            .entries
            .into_iter()
            .filter(|w| w.folder_uri.is_some())
            .map(|w| Workspace::new(w.folder_uri.unwrap(), icon.clone()))
            .collect();

        Ok(())
    }

    fn query_direct(
        &mut self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
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
    icon: Icon,
}

impl Workspace {
    fn new(uri: url::Url, icon: Icon) -> Self {
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

        Self {
            uri,
            name,
            path,
            icon,
        }
    }
}

impl Workspace {
    fn item(&self, score: u16) -> ResultItem {
        let uri = self.uri.clone();

        ResultItem {
            id: format!("{}:{}", Plugin::NAME, self.name),
            icon: self.icon.clone(),
            primary_text: self.name.clone(),
            secondary_text: self.path.to_string_lossy().to_string(),
            tooltip: None,
            actions: vec![Action::primary(move |_| {
                kal_utils::execute_with_args(
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
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<ResultItem> {
        matcher
            .fuzzy_match(&self.name, query)
            .map(|score| self.item(score))
    }
}
