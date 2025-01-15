use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::icon::{self, BuiltInIcon, Icon};
use crate::search_result_item::{IntoSearchResultItem, SearchResultItem};
use crate::utils::{self, thread, IteratorExt, PathExt, ResolveEnvVars};

#[derive(Debug)]
struct DirEntry {
    name: OsString,
    path: PathBuf,
    icon: Option<PathBuf>,
    id: String,
}

impl DirEntry {
    fn new(path: PathBuf, icons_dir: &Path) -> Self {
        let name = path.file_stem().unwrap_or_default().to_os_string();
        let filename = path.file_name().unwrap_or_default().to_os_string();
        let is_file = path.is_file();
        let icon = is_file.then(|| icons_dir.join(&filename).with_extra_extension("png"));
        let id = format!("{}:{}", Plugin::NAME, filename.to_string_lossy());
        Self {
            name,
            path,
            icon,
            id,
        }
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        utils::execute(&self.path, elevated)
    }

    fn show_item_in_dir(&self) -> anyhow::Result<()> {
        utils::reveal_in_dir(&self.path)
    }
}

impl IntoSearchResultItem for DirEntry {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<SearchResultItem> {
        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .or_else(|| matcher.fuzzy_match(&self.path.to_string_lossy(), query))
            .map(|score| SearchResultItem {
                primary_text: self.name.to_string_lossy(),
                secondary_text: self.path.to_string_lossy(),
                icon: match &self.icon {
                    Some(path) => Icon::path(path.to_string_lossy()),
                    None => BuiltInIcon::Directory.icon(),
                },
                needs_confirmation: false,
                id: self.id.as_str().into(),
                score,
            })
    }
}

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

    fn find_dirs(&mut self) {
        self.entries = self
            .paths
            .iter()
            .map(ResolveEnvVars::resolve_vars)
            .filter_map(|path| read_dir(path).ok())
            .flatten()
            .map(|e| DirEntry::new(e.path(), &self.icons_dir))
            .collect::<Vec<DirEntry>>();
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config, data_dir: &Path) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Self {
            paths: config.paths,
            icons_dir: data_dir.join("icons"),
            entries: Vec::new(),
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        self.update_config(config);
        self.find_dirs();

        let icons_dir = self.icons_dir.clone();
        let paths = self
            .entries
            .iter()
            .filter_map(|e| e.icon.as_ref().map(|icon| (e.path.clone(), icon.clone())))
            .collect::<Vec<_>>();

        thread::spawn(move || {
            std::fs::create_dir_all(icons_dir)?;
            icon::extract_multiple(paths)
        });

        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
        Ok(self
            .entries
            .iter()
            .filter_map(|entry| entry.fuzzy_match(query, matcher))
            .collect_non_empty())
    }

    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
            entry.execute(elevated)?;
        }
        Ok(())
    }

    fn show_item_in_dir(&self, id: &str) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
            entry.show_item_in_dir()?;
        }
        Ok(())
    }
}

fn read_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<fs::DirEntry>> {
    let entries = fs::read_dir(path)?;
    let entries = entries
        .flatten()
        .filter_map(|e| {
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
        })
        .collect();

    Ok(entries)
}
