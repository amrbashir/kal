use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
    utils::{self, thread},
    KAL_DATA_DIR,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

const PLUGIN_NAME: &str = "DirectoryIndexer";

#[derive(Debug)]
struct DirEntry {
    name: OsString,
    path: PathBuf,
    icon: Option<PathBuf>,
    identifier: String,
}

impl<'a> From<&'a DirEntry> for SearchResultItem<'a> {
    fn from(dir: &'a DirEntry) -> Self {
        Self {
            primary_text: dir.name.to_string_lossy(),
            secondary_text: dir.path.to_string_lossy(),
            icon: match &dir.icon {
                Some(path) => Icon::path(path.to_string_lossy()),
                None => Defaults::Directory.icon(),
            },
            needs_confirmation: false,
            identifier: dir.identifier.as_str().into(),
        }
    }
}

impl DirEntry {
    fn new(path: PathBuf, icons_dir: &Path) -> Self {
        let name = path.file_stem().unwrap_or_default().to_os_string();
        let identifier = format!("{PLUGIN_NAME}:{}", name.to_string_lossy());
        let is_file = path.is_file();
        let icon = is_file.then(|| icons_dir.join(&name).with_extension("png"));
        Self {
            name,
            path,
            icon,
            identifier,
        }
    }

    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> bool {
        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .is_some()
            || matcher
                .fuzzy_match(&self.path.to_string_lossy(), query)
                .is_some()
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        utils::execute(&self.path, elevated)
    }
    fn reveal_in_dir(&self) -> anyhow::Result<()> {
        utils::reveal_in_dir(&self.path)
    }
}

#[derive(Debug)]
pub struct Plugin {
    paths: Vec<String>,

    icons_dir: PathBuf,
    entries: Vec<DirEntry>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    enabled: bool,
    paths: Vec<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            paths: Default::default(),
        }
    }
}

impl Plugin {
    const NAME: &'static str = "DirectoryIndexer";

    fn name(&self) -> &str {
        Self::NAME
    }

    fn update_config(&mut self, config: &Config) {
        let config = config.plugin_config::<PluginConfig>(self.name());
        self.paths = config.paths;
    }

    fn find_dirs(&mut self) {
        self.entries = self
            .paths
            .iter()
            .map(utils::resolve_env_vars)
            .filter_map(|path| read_dir(path).ok())
            .flatten()
            .map(|e| DirEntry::new(e.path(), &self.icons_dir))
            .collect::<Vec<DirEntry>>();
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Self {
            paths: config.paths,
            icons_dir: KAL_DATA_DIR.join("icons"),
            entries: Vec::new(),
        })
    }

    fn name(&self) -> &str {
        self.name()
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
            // TODO: automatic invalidation based on hash?
            // or after a period of time? we should avoid
            // regeneratin the icons on each app start and reload
            utils::extract_pngs(paths)
        });

        Ok(())
    }

    fn results(
        &self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Vec<SearchResultItem<'_>>> {
        let filtered = self
            .entries
            .iter()
            .filter(|entry| entry.fuzzy_match(query, matcher))
            .map(Into::into)
            .collect::<Vec<_>>();

        Ok(filtered)
    }

    fn execute(&self, identifier: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.identifier == identifier) {
            entry.execute(elevated)?;
        }
        Ok(())
    }

    fn reveal_in_dir(&self, identifier: &str) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.identifier == identifier) {
            entry.reveal_in_dir()?;
        }
        Ok(())
    }
}

fn read_dir<P: AsRef<Path>>(path: P) -> anyhow::Result<Vec<fs::DirEntry>> {
    let entries = fs::read_dir(path)?;
    let entries = entries
        .flatten()
        .filter_map(|e| {
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
