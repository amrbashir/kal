use crate::{
    common_types::{Icon, IconType, SearchResultItem},
    plugin::impl_plugin,
    plugin::Plugin,
};
use std::{fs, path};

#[cfg(target_os = "windows")]
#[path = "windows.rs"]
mod platform;
#[cfg(target_os = "linux")]
#[path = "linux.rs"]
mod platform;
#[path = "macos.rs"]
#[cfg(target_os = "macos")]
mod platform;

pub struct AppLauncherPlugin {
    name: String,
    paths: Vec<String>,
    extensions: Vec<String>,
    cached_apps: Vec<SearchResultItem>,
}

impl_plugin!(AppLauncherPlugin);

impl AppLauncherPlugin {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            name: "AppLauncherPlugin".to_string(),
            // TODO load these from config
            paths: vec![
                "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
                "C:\\Users\\amr\\AppData\\Roaming\\Microsoft\\Windows\\Start Menu".to_string(),
                "C:\\Users\\amr\\Desktop".to_string(),
                "D:\\Games".to_string(),
            ],
            extensions: vec![
                "lnk".to_string(),
                "appref-ms".to_string(),
                "exe".to_string(),
                "cmd".to_string(),
                "bat".to_string(),
                "py".to_string(),
            ],
            cached_apps: Vec::new(),
        })
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
