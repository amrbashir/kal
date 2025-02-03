use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use kal_config::Config;
use kal_plugin::{Action, Icon, IntoResultItem, PluginQueryOutput, ResultItem};
use kal_utils::{ExpandEnvVars, IteratorExt};
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct Plugin {
    paths: Vec<String>,
    entries: Vec<IndexedEntry>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
#[serde(deny_unknown_fields)]
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

    fn index_dirs(&mut self) {
        self.entries = self
            .paths
            .iter()
            .map(ExpandEnvVars::expand_vars)
            .map(read_dir)
            .flatten()
            .flatten()
            .map(|e| IndexedEntry::new(e.path()))
            .collect()
    }
}

impl kal_plugin::Plugin for Plugin {
    fn new(config: &Config) -> Self {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Self {
            paths: config.paths,
            entries: Vec::new(),
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        self.update_config(config);
        self.index_dirs();
        Ok(())
    }

    fn query(
        &mut self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
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
struct IndexedEntry {
    name: OsString,
    path: PathBuf,
    is_dir: bool,
    id: String,
}

impl IndexedEntry {
    fn new(path: PathBuf) -> Self {
        let name = path.file_stem().unwrap_or_default().to_os_string();
        let filename = path.file_name().unwrap_or_default().to_os_string();
        let is_dir = path.is_dir();
        let id = format!("{}:{}", Plugin::NAME, filename.to_string_lossy());
        Self {
            name,
            is_dir,
            path,
            id,
        }
    }

    fn item(&self, score: u16) -> ResultItem {
        let actions = if self.is_dir {
            vec![
                {
                    let path = self.path.clone();
                    Action::primary(move |_| kal_utils::open_dir(&path))
                },
                {
                    let path = self.path.clone();
                    Action::open_location(move |_| kal_utils::reveal_item_in_dir(&path))
                },
            ]
        } else {
            vec![
                {
                    let path = self.path.clone();
                    Action::primary(move |_| kal_utils::execute(&path, false))
                },
                {
                    let path = self.path.clone();
                    Action::open_elevated(move |_| kal_utils::execute(&path, true))
                },
                {
                    let path = self.path.clone();
                    Action::open_location(move |_| kal_utils::reveal_item_in_dir(&path))
                },
            ]
        };

        ResultItem {
            id: self.id.as_str().into(),
            icon: Icon::extract_path(self.path.to_string_lossy()),
            primary_text: self.name.to_string_lossy().into_owned(),
            secondary_text: self.path.to_string_lossy().into_owned(),
            tooltip: None,
            actions,
            score,
        }
    }
}

impl IntoResultItem for IndexedEntry {
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<ResultItem> {
        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .or_else(|| matcher.fuzzy_match(&self.path.to_string_lossy(), query))
            .map(|score| self.item(score))
    }
}

fn read_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<impl Iterator<Item = fs::DirEntry>> {
    let entries = fs::read_dir(path)?.flatten().filter_map(|e| {
        // skip hidden files and directories on Windows
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;

            use windows::Win32::Storage::FileSystem::FILE_ATTRIBUTE_HIDDEN;
            if e.metadata()
                .map(|m| (m.file_attributes() & FILE_ATTRIBUTE_HIDDEN.0) != 0)
                .unwrap_or(false)
            {
                return None;
            }
        }

        Some(e)
    });

    Ok(entries)
}
