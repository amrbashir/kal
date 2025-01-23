use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use smol::stream::*;

use crate::config::Config;
use crate::icon::{self, Icon};
use crate::plugin::PluginQueryOutput;
use crate::result_item::{Action, IntoResultItem, ResultItem};
use crate::utils::{self, ExpandEnvVars, IteratorExt, PathExt};

#[derive(Debug)]
pub struct Plugin {
    paths: Vec<String>,
    icons_dir: PathBuf,
    entries: Vec<DirEntry>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PluginConfig {
    #[serde(default)]
    paths: Vec<String>,
}

impl Plugin {
    const NAME: &'static str = "DirectoryIndexer";

    fn update_config(&mut self, config: &Config) {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.paths = config.paths;
    }

    async fn index_dirs(&mut self) {
        self.entries.clear();

        let expanded_paths = self.paths.iter().map(ExpandEnvVars::expand_vars);
        let mut entries = smol::stream::iter(expanded_paths).map(read_dir);

        while let Some(e) = entries.next().await {
            if let Ok(e) = e.await {
                let e = e.iter().map(|e| DirEntry::new(e.path(), &self.icons_dir));
                self.entries.extend(e);
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config, data_dir: &Path) -> Self {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Self {
            paths: config.paths,
            icons_dir: data_dir.join("icons"),
            entries: Vec::new(),
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        self.update_config(config);

        self.index_dirs().await;

        let icons_dir = self.icons_dir.clone();
        let paths = self
            .entries
            .iter()
            .map(|e| (e.path.clone(), e.icon.clone()))
            .collect::<Vec<_>>();

        smol::fs::create_dir_all(icons_dir).await?;
        let _ = icon::extract_multiple_cached(paths).inspect_err(|e| tracing::error!("{e}"));

        Ok(())
    }

    async fn query(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<PluginQueryOutput> {
        Ok(self
            .entries
            .iter()
            .filter_map(|entry| entry.fuzzy_match(query, matcher))
            .collect_non_empty::<Vec<_>>()
            .into())
    }
}

#[derive(Debug)]
struct DirEntry {
    name: OsString,
    path: PathBuf,
    is_dir: bool,
    icon: PathBuf,
    id: String,
}

impl DirEntry {
    fn new(path: PathBuf, icons_dir: &Path) -> Self {
        let name = path.file_stem().unwrap_or_default().to_os_string();
        let filename = path.file_name().unwrap_or_default().to_os_string();
        let is_dir = path.is_dir();
        let icon = icons_dir.join(&filename).with_extra_extension("png");
        let id = format!("{}:{}", Plugin::NAME, filename.to_string_lossy());
        Self {
            name,
            is_dir,
            path,
            icon,
            id,
        }
    }

    fn item(&self, score: i64) -> ResultItem {
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

        let tooltip = format!("{}\n{}", self.name.to_string_lossy(), self.path.display());

        ResultItem {
            id: self.id.as_str().into(),
            icon: Icon::path(self.icon.to_string_lossy()),
            primary_text: self.name.to_string_lossy().into_owned(),
            secondary_text: self.path.to_string_lossy().into_owned(),
            tooltip: Some(tooltip),
            actions,
            score,
        }
    }
}

impl IntoResultItem for DirEntry {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem> {
        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .or_else(|| matcher.fuzzy_match(&self.path.to_string_lossy(), query))
            .map(|score| self.item(score))
    }
}

async fn read_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<smol::fs::DirEntry>> {
    let mut entries = smol::fs::read_dir(path).await?;
    let mut out = Vec::with_capacity(entries.size_hint().0);

    while let Some(entry) = entries.try_next().await? {
        // skip hidden files and directories on Windows
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;

            use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
            if entry
                .metadata()
                .await
                .map(|m| (m.file_attributes() & FILE_ATTRIBUTE_HIDDEN.0) != 0)
                .unwrap_or(false)
            {
                continue;
            }
        }

        out.push(entry);
    }

    Ok(out)
}
