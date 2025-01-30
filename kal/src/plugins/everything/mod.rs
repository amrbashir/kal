use std::ffi::OsString;
use std::path::PathBuf;


use serde::{Deserialize, Serialize};

use crate::config::{Config, GenericPluginConfig};
use crate::icon::Icon;
use crate::plugin::PluginQueryOutput;
use crate::result_item::{Action, IntoResultItem, ResultItem};
use crate::utils::{self};

#[derive(Debug)]
pub struct Plugin {
    es: PathBuf,
    max_results: usize,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PluginConfig {
    es: Option<PathBuf>,
}

impl Plugin {
    const NAME: &'static str = "Everything";
}

#[async_trait::async_trait]
impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> Self {
        let max_results = config.general.max_results;
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Self {
            es: config.es.unwrap_or_else(|| PathBuf::from("es")),
            max_results,
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(false),
            direct_activation_command: Some("?".into()),
        }
    }

    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        self.max_results = config.general.max_results;
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.es = config.es.unwrap_or_else(|| PathBuf::from("es"));
        Ok(())
    }

    async fn query_direct(
        &mut self,
        query: &str,
        matcher: &mut crate::fuzzy_matcher::Matcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        if query.is_empty() {
            return Ok(PluginQueryOutput::None);
        }

        let output = std::process::Command::new(&self.es)
            .arg(query)
            .arg("-n")
            .arg(self.max_results.to_string())
            .output()?;

        match output.status.success() {
            true => {}
            false => {
                let stderr = std::str::from_utf8(&output.stderr).unwrap_or_default();
                anyhow::bail!("{stderr}");
            }
        }

        let output = String::from_utf8_lossy(output.stdout.as_slice());

        let entries = output.lines().map(EverythingEntry::new).collect::<Vec<_>>();

        Ok(entries
            .iter()
            .filter_map(|e| e.fuzzy_match(query, matcher))
            .collect::<Vec<_>>()
            .into())
    }
}

#[derive(Debug)]
struct EverythingEntry {
    name: OsString,
    path: PathBuf,
    is_dir: bool,
    id: String,
}

impl EverythingEntry {
    fn new(path: &str) -> Self {
        let path = PathBuf::from(path);
        let name = path.file_name().unwrap_or_default().to_os_string();
        let is_dir = path.is_dir();
        let id = format!("{}:{}", Plugin::NAME, name.to_string_lossy());
        Self {
            name,
            path,
            id,
            is_dir,
        }
    }
}

impl IntoResultItem for EverythingEntry {
    fn fuzzy_match(&self, _query: &str, _matcher: &mut crate::fuzzy_matcher::Matcher) -> Option<ResultItem> {
        let actions = if self.is_dir {
            vec![
                {
                    let path = self.path.clone();
                    Action::primary(move |_| utils::open_dir(&path))
                },
                {
                    let path = self.path.clone();
                    Action::open_location(move |_| utils::reveal_item_in_dir(&path))
                },
            ]
        } else {
            vec![
                {
                    let path = self.path.clone();
                    Action::primary(move |_| utils::execute(&path, false))
                },
                {
                    let path = self.path.clone();
                    Action::open_elevated(move |_| utils::execute(&path, true))
                },
                {
                    let path = self.path.clone();
                    Action::open_location(move |_| utils::reveal_item_in_dir(&path))
                },
            ]
        };

        Some(ResultItem {
            id: self.id.as_str().into(),
            icon: Icon::extract_path(self.path.to_string_lossy()),
            primary_text: self.name.to_string_lossy().into_owned(),
            secondary_text: self.path.to_string_lossy().into_owned(),
            tooltip: None,
            actions,
            score: 0,
        })
    }
}
