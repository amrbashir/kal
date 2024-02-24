use crate::{
    common::{
        icon::{Defaults, Icon, IconKind},
        SearchResultItem,
    },
    config::Config,
    utils::{self, thread},
    KAL_DATA_DIR,
};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
pub struct Plugin {
    name: String,
    enabled: bool,
    paths: Vec<String>,
    cached_dir_entries: Vec<SearchResultItem>,
    icons_dir: PathBuf,
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

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Box<Self>> {
        let name = "DirectoryIndexer".to_string();
        let config = config.plugin_config::<PluginConfig>(&name);

        Ok(Box::new(Self {
            name,
            enabled: config.enabled,
            paths: config.paths,
            cached_dir_entries: Vec::new(),
            icons_dir: KAL_DATA_DIR.join("icons"),
        }))
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(&self.name);
        self.enabled = config.enabled;
        self.paths = config.paths;

        let dir_entries = self
            .paths
            .iter()
            .filter_map(|path| {
                let path = utils::resolve_env_vars(path);
                read_dir(path).ok()
            })
            .flatten()
            .map(|e| {
                let file = e.path();

                let icon = if e.metadata().map(|e| e.is_dir()).unwrap_or(false) {
                    Defaults::Folder.icon()
                } else {
                    let p = self
                        .icons_dir
                        .join(file.file_stem().unwrap_or_default())
                        .with_extension("png");
                    Icon {
                        data: p.to_string_lossy().into_owned(),
                        kind: IconKind::Path,
                    }
                };

                let app_name = file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let path = file.to_string_lossy().into_owned();

                SearchResultItem {
                    primary_text: app_name,
                    secondary_text: path.clone(),
                    execution_args: serde_json::Value::String(path),
                    plugin_name: self.name.clone(),
                    icon,
                    needs_confirmation: false,
                }
            })
            .collect::<Vec<SearchResultItem>>();

        self.cached_dir_entries = dir_entries.clone();

        let _ = std::fs::create_dir_all(&self.icons_dir);
        thread::spawn(move || {
            utils::extract_pngs(dir_entries.into_iter().filter_map(|i| {
                if i.icon.kind == IconKind::Path {
                    Some(i)
                } else {
                    None
                }
            }))
        });

        Ok(())
    }

    fn results(&self, _query: &str) -> anyhow::Result<&[SearchResultItem]> {
        Ok(&self.cached_dir_entries)
    }

    fn execute(&self, item: &SearchResultItem, elevated: bool) -> anyhow::Result<()> {
        let path = item.path()?;
        utils::execute(path, elevated);
        Ok(())
    }

    fn open_location(&self, item: &SearchResultItem) -> anyhow::Result<()> {
        let path = item.path()?;
        if let Some(parent) = path.parent() {
            utils::open_path(parent);
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
