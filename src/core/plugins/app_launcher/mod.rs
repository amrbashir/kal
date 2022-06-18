use crate::{
    common::{Icon, IconType, SearchResultItem},
    config::Config,
    plugin::impl_plugin,
};
use serde::{Deserialize, Serialize};
use std::{fs, path, thread};

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;

#[derive(Debug)]
pub struct AppLauncherPlugin {
    name: String,
    enabled: bool,
    paths: Vec<String>,
    extensions: Vec<String>,
    cached_apps: Vec<SearchResultItem>,
    cache_path: path::PathBuf,
}

#[derive(Serialize, Deserialize, Debug)]
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
            cache_path: dirs_next::data_local_dir()
                .expect("Failed to get $data_local_dir path")
                .join("kal")
                .join("cache"),
        })
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn name(&self) -> &str {
        &self.name
    }
    pub fn refresh(&mut self) {
        let mut apps = Vec::new();
        for path in &self.paths {
            apps.extend(filter_path_entries_by_extensions(
                path::PathBuf::from(path),
                &self.extensions,
            ));
        }

        self.cached_apps = apps
            .iter()
            .enumerate()
            .map(|(i, e)| {
                let file = e.path();

                let mut icon_path = self.cache_path.join(format!(
                    "{}-{}",
                    file.file_stem().unwrap_or_default().to_string_lossy(),
                    i.to_string()
                )); // to avoid collision if a file with the same file stem exists in two different places
                icon_path.set_extension("png");

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
                        data: icon_path.to_string_lossy().into_owned(),
                        r#type: IconType::Path,
                    },
                }
            })
            .collect::<Vec<SearchResultItem>>();

        let _ = std::fs::create_dir_all(&self.cache_path);
        let apps = self.cached_apps.clone();
        thread::spawn(move || {
            let _ = platform::extract_png(
                apps.into_iter()
                    .map(|a| (a.execution_args[0].clone(), a.icon.data.clone()))
                    .collect(),
            );
        });
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
