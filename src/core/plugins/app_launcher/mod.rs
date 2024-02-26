use crate::{
    common::{icon::Icon, SearchResultItem},
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
    enabled: bool,
    paths: Vec<String>,
    extensions: Vec<String>,
    cached_apps: Vec<SearchResultItem>,
    icons_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    enabled: bool,
    paths: Vec<String>,
    extensions: Vec<String>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            paths: vec![
                "%PROGRAMDATA%\\Microsoft\\Windows\\Start Menu".to_string(),
                "%APPDATA%\\Microsoft\\Windows\\Start Menu".to_string(),
                "%USERPROFILE%\\Desktop".to_string(),
            ],
            extensions: vec!["exe".to_string(), "lnk".to_string()],
        }
    }
}

impl Plugin {
    const NAME: &'static str = "AppLauncher";

    fn name(&self) -> &str {
        Self::NAME
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Box<Self>> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Box::new(Self {
            enabled: config.enabled,
            paths: config.paths,
            extensions: config.extensions,
            cached_apps: Vec::new(),
            icons_dir: KAL_DATA_DIR.join("icons"),
        }))
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(self.name());
        self.enabled = config.enabled;
        self.paths = config.paths;
        self.extensions = config.extensions;

        let apps = self
            .paths
            .iter()
            .filter_map(|path| {
                let path = utils::resolve_env_vars(path);
                filter_path_entries_by_extensions(path, &self.extensions).ok()
            })
            .flatten()
            .map(|e| {
                let file = e.path();

                let icon_path = self
                    .icons_dir
                    .join(file.file_stem().unwrap_or_default())
                    .with_extension("png");

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
                    plugin_name: self.name().to_string(),
                    icon: Icon::path(icon_path.to_string_lossy().into_owned()),
                    needs_confirmation: false,
                }
            })
            .collect::<Vec<SearchResultItem>>();

        self.cached_apps = apps.clone();

        std::fs::create_dir_all(&self.icons_dir)?;
        thread::spawn(move || utils::extract_pngs(apps));

        Ok(())
    }

    fn results(&self, _query: &str) -> anyhow::Result<&[SearchResultItem]> {
        Ok(&self.cached_apps)
    }

    fn execute(&self, item: &SearchResultItem, elevated: bool) -> anyhow::Result<()> {
        let app = item.path()?;
        utils::execute(app, elevated);
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

fn filter_path_entries_by_extensions<P>(
    path: P,
    extensions: &[String],
) -> anyhow::Result<Vec<fs::DirEntry>>
where
    P: AsRef<Path>,
{
    let mut filtered = Vec::new();

    let entries = fs::read_dir(path)?;
    for entry in entries.flatten() {
        if let Ok(metadata) = entry.metadata() {
            if metadata.is_file() {
                let path = entry.path();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if extensions.contains(&extension) {
                    filtered.push(entry);
                }
            } else {
                let filtered_entries = filter_path_entries_by_extensions(entry.path(), extensions)?;
                filtered.extend(filtered_entries);
            }
        }
    }

    Ok(filtered)
}
