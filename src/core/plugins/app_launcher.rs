use crate::{
    common_types::{Icon, IconType, SearchResultItem},
    plugin::Plugin,
};
use std::{
    fs::{self, DirEntry},
    path::PathBuf,
};

pub struct AppLauncherPlugin {
    name: String,
    paths: Vec<String>,
    extensions: Vec<String>,
    cached_apps: Vec<SearchResultItem>,
}

impl AppLauncherPlugin {
    pub fn new() -> Box<Self> {
        Box::new(Self {
            name: "AppLauncherPlugin".to_string(),
            paths: vec![
                "C:\\ProgramData\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
                "E:\\Scripts".to_string(),
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
    pub fn refresh(&mut self) {
        let mut filtered_entries = Vec::new();
        for path in &self.paths {
            filtered_entries.extend(filter_path_entries_by_extensions(
                PathBuf::from(path),
                &self.extensions,
            ));
        }

        self.cached_apps = filtered_entries
            .iter()
            .map(|e| {
                let path = e.path();
                let path_str = path.to_str().unwrap_or_default().to_string();
                SearchResultItem {
                    primary_text: path
                        .file_stem()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                        .to_string(),
                    secondary_text: path_str.clone(),
                    execution_args: vec![path_str],
                    plugin_name: self.name.clone(),
                    icon: Icon {
                        value: "dmmy.png".into(),
                        r#type: IconType::Path,
                    },
                }
            })
            .collect::<Vec<SearchResultItem>>();
    }
    pub fn results(&self, _query: &str) -> &[SearchResultItem] {
        &self.cached_apps
    }

    pub fn execute(&self, item: &SearchResultItem) {
        #[cfg(target_os = "windows")]
        {
            use std::ptr::null;
            use windows_sys::Win32::UI::Shell::ShellExecuteW;
            const SW_SHOWNORMAL: u32 = 1u32;
            unsafe {
                ShellExecuteW(
                    null::<isize>() as _,
                    null(),
                    encode_wide(&item.execution_args[0]).as_ptr(),
                    null(),
                    null(),
                    SW_SHOWNORMAL as _,
                )
            };
        }
    }
}

fn filter_path_entries_by_extensions(path: PathBuf, extensions: &Vec<String>) -> Vec<DirEntry> {
    let mut filtered = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        let entries = entries
            .filter_map(|e| if e.is_ok() { Some(e.unwrap()) } else { None })
            .collect::<Vec<DirEntry>>();
        for entry in entries {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_file() {
                    if extensions.contains(
                        &entry
                            .path()
                            .extension()
                            .unwrap_or_default()
                            .to_str()
                            .unwrap_or_default()
                            .to_string(),
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

#[cfg(target_os = "windows")]
fn encode_wide(string: impl AsRef<std::ffi::OsStr>) -> Vec<u16> {
    std::os::windows::prelude::OsStrExt::encode_wide(string.as_ref())
        .chain(std::iter::once(0))
        .collect()
}

impl Plugin for AppLauncherPlugin {
    fn name(&self) -> &str {
        &self.name
    }

    fn refresh(&mut self) {
        self.refresh()
    }

    fn results(&self, query: &str) -> &[SearchResultItem] {
        self.results(query)
    }

    fn execute(&self, item: &SearchResultItem) {
        self.execute(item)
    }
}
