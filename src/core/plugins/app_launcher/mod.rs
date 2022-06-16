use crate::{
    common::{Icon, IconType, SearchResultItem},
    config::Config,
    plugin::impl_plugin,
};
use serde::{Deserialize, Serialize};
use std::{fs, path};

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;

pub struct AppLauncherPlugin {
    name: String,
    enabled: bool,
    paths: Vec<String>,
    extensions: Vec<String>,
    cached_apps: Vec<SearchResultItem>,
}

#[derive(Serialize, Deserialize)]
struct AppLauncherPluginConfig {
    enabled: bool,
    paths: Vec<String>,
    extensions: Vec<String>,
}

impl Default for AppLauncherPluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            paths: Default::default(),
            extensions: Default::default(),
        }
    }
}

impl_plugin!(AppLauncherPlugin);

impl AppLauncherPlugin {
    pub fn new(config: &Config) -> Box<Self> {
        let name = "AppLauncher".to_string();
        let config = config
            .plugin_config::<AppLauncherPluginConfig>(&name)
            .unwrap_or_default();
        Box::new(Self {
            name,
            enabled: config.enabled,
            paths: config.paths,
            extensions: config.extensions,
            cached_apps: Vec::new(),
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn refresh(&mut self) {
        let mut filtered_entries = Vec::new();
        for path in &self.paths {
            filtered_entries.extend(filter_path_entries_by_extensions(
                path::PathBuf::from(path),
                &self.extensions,
            ));
        }

        self.cached_apps = filtered_entries
            .iter()
            .map(|e| {
                let file = e.path();

                let mut cache = dirs_next::home_dir().expect("Failed to get $HOME dir path");
                cache.push(".kal");
                cache.push("cache");
                let _ = fs::create_dir_all(&cache);

                let mut icon = cache.clone();
                icon.push(file.file_stem().unwrap_or_default());
                icon.set_extension("png");

                let _ = platform::extract_png(&file, &icon);

                let app_name = file
                    .file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                let path = file.to_string_lossy().into_owned();
                SearchResultItem {
                    primary_text: app_name,
                    secondary_text: path.clone(),
                    execution_args: vec![path],
                    plugin_name: self.name.clone(),
                    icon: Icon {
                        data: icon.to_string_lossy().into_owned(),
                        r#type: IconType::Path,
                    },
                }
            })
            .collect::<Vec<SearchResultItem>>();
    }
    pub fn results(&self, _query: &str) -> &[SearchResultItem] {
        &self.cached_apps
    }

    pub fn execute(&self, item: &SearchResultItem, elevated: bool) {
        platform::execute(item, elevated);
    }

    pub fn open_location(&self, item: &SearchResultItem) {
        platform::open_location(item);
    }
}

fn filter_path_entries_by_extensions<P: AsRef<path::Path>>(
    path: P,
    extensions: &Vec<String>,
) -> Vec<fs::DirEntry> {
    let mut filtered = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        let entries = entries
            .filter_map(|e| if let Ok(e) = e { Some(e) } else { None })
            .collect::<Vec<fs::DirEntry>>();
        for entry in entries {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if extensions.contains(
                        &entry
                            .path()
                            .extension()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                    ) {
                        filtered.push(entry);
                    }
                } else {
                    let filtered_entries =
                        filter_path_entries_by_extensions(entry.path(), extensions);
                    filtered.extend(filtered_entries);
                }
            }
        }
    }

    filtered
}
