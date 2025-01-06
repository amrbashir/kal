use crate::{
    common::{icon::Icon, IntoSearchResultItem, SearchResultItem},
    config::Config,
    utils::{self, thread, IteratorExt, PathExt, ResolveEnvVars},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
};

#[derive(Debug)]
struct App {
    name: OsString,
    path: PathBuf,
    icon: PathBuf,
    id: String,
}

impl App {
    fn new(path: PathBuf, icons_dir: &Path) -> Self {
        let name = path.file_stem().unwrap_or_default().to_os_string();
        let filename = path.file_name().unwrap_or_default().to_os_string();
        let icon = icons_dir.join(&filename).with_extra_extension("png");
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

    fn reveal_in_dir(&self) -> anyhow::Result<()> {
        utils::reveal_in_dir(&self.path)
    }
}

impl IntoSearchResultItem for App {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<SearchResultItem> {
        matcher
            .fuzzy_match(&self.name.to_string_lossy(), query)
            .or_else(|| matcher.fuzzy_match(&self.path.to_string_lossy(), query))
            .map(|score| SearchResultItem {
                primary_text: self.name.to_string_lossy(),
                secondary_text: self.path.to_string_lossy(),
                icon: Icon::path(self.icon.to_string_lossy()),
                needs_confirmation: false,
                id: self.id.as_str().into(),
                score,
            })
    }
}

#[derive(Debug)]
pub struct Plugin {
    paths: Vec<String>,
    extensions: Vec<String>,

    icons_dir: PathBuf,
    apps: Vec<App>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    #[serde(default = "default_paths")]
    paths: Vec<String>,
    #[serde(default = "default_extensions")]
    extensions: Vec<String>,
}

fn default_paths() -> Vec<String> {
    vec![
        "%USERPROFILE%\\Desktop".to_string(),
        "%APPDATA%\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
        "%PROGRAMDATA%\\Microsoft\\Windows\\Start Menu\\Programs".to_string(),
    ]
}

fn default_extensions() -> Vec<String> {
    vec!["exe".to_string(), "lnk".to_string()]
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            paths: default_paths(),
            extensions: default_extensions(),
        }
    }
}

impl Plugin {
    const NAME: &'static str = "AppLauncher";

    fn update_config(&mut self, config: &Config) {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.paths = config.paths;
        self.extensions = config.extensions;
    }

    fn find_apps(&mut self) {
        self.apps = self
            .paths
            .iter()
            .map(ResolveEnvVars::resolve_vars)
            .filter_map(|p| filter_path_entries_by_extensions(p, &self.extensions).ok())
            .flatten()
            .map(|e| App::new(e.path(), &self.icons_dir))
            .collect::<Vec<App>>();
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config, data_dir: &Path) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        Ok(Self {
            paths: config.paths,
            extensions: config.extensions,
            icons_dir: data_dir.join("icons"),
            apps: Vec::new(),
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        self.update_config(config);
        self.find_apps();

        let icons_dir = self.icons_dir.clone();
        let paths = self
            .apps
            .iter()
            .map(|app| (app.path.clone(), app.icon.clone()))
            .collect::<Vec<_>>();

        thread::spawn(move || {
            std::fs::create_dir_all(icons_dir)?;
            utils::extract_icons(paths)
        });

        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
        Ok(self
            .apps
            .iter()
            .filter_map(|app| app.fuzzy_match(query, matcher))
            .collect_non_empty())
    }

    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(app) = self.apps.iter().find(|app| app.id == id) {
            app.execute(elevated)?;
        }
        Ok(())
    }

    fn reveal_in_dir(&self, id: &str) -> anyhow::Result<()> {
        if let Some(app) = self.apps.iter().find(|app| app.id == id) {
            app.reveal_in_dir()?;
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
            if metadata.is_dir() {
                let filtered_entries = filter_path_entries_by_extensions(entry.path(), extensions)?;
                filtered.extend(filtered_entries);
            } else {
                let path = entry.path();
                let extension = path
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .to_string();
                if extensions.contains(&extension) {
                    filtered.push(entry);
                }
            }
        }
    }

    Ok(filtered)
}
