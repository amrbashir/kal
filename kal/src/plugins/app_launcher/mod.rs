use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::icon::{self};
use crate::result_item::{IntoResultItem, ResultItem};
use crate::utils::IteratorExt;

mod packaged_app;
mod program;

#[derive(Debug)]
enum App {
    Program(program::Program),
    Packaged(packaged_app::PackagedApp),
}

impl App {
    fn id(&self) -> &str {
        match self {
            App::Program(program) => &program.id,
            App::Packaged(packaged_app) => &packaged_app.id,
        }
    }

    fn icon_path(&self) -> Option<(PathBuf, PathBuf)> {
        match self {
            App::Program(program) => Some((program.path.clone(), program.icon.clone())),
            App::Packaged(_) => None,
        }
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        match self {
            App::Program(program) => program.execute(elevated),
            App::Packaged(packaged_app) => packaged_app.execute(elevated),
        }
    }

    fn show_item_in_dir(&self) -> anyhow::Result<()> {
        match self {
            App::Program(program) => program.show_item_in_dir(),
            App::Packaged(_) => Ok(()),
        }
    }
}

impl IntoResultItem for App {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem> {
        match self {
            App::Program(program) => program.fuzzy_match(query, matcher),
            App::Packaged(packaged_app) => packaged_app.fuzzy_match(query, matcher),
        }
    }
}

#[derive(Debug)]
pub struct Plugin {
    paths: Vec<String>,
    extensions: Vec<String>,
    include_packaged_apps: bool,
    icons_dir: PathBuf,
    apps: Vec<App>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    #[serde(default = "default_paths")]
    paths: Vec<String>,
    #[serde(default = "default_extensions")]
    extensions: Vec<String>,
    #[serde(default = "default_include_packaged_apps")]
    include_packaged_apps: bool,
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

fn default_include_packaged_apps() -> bool {
    true
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            paths: default_paths(),
            extensions: default_extensions(),
            include_packaged_apps: default_include_packaged_apps(),
        }
    }
}

impl Plugin {
    const NAME: &'static str = "AppLauncher";

    fn update_config(&mut self, config: &Config) {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.paths = config.paths;
        self.extensions = config.extensions;
        self.include_packaged_apps = config.include_packaged_apps;
    }

    fn find_apps(&mut self) {
        self.apps = program::find_all_in_paths(&self.paths, &self.extensions, &self.icons_dir)
            .map(App::Program)
            .collect();

        if self.include_packaged_apps {
            if let Ok(packaged_apps) = packaged_app::find_all() {
                self.apps.extend(packaged_apps.map(App::Packaged));
            }
        }
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config, data_dir: &Path) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        Ok(Self {
            paths: config.paths,
            extensions: config.extensions,
            include_packaged_apps: config.include_packaged_apps,
            icons_dir: data_dir.join("icons"),
            apps: Vec::new(),
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        self.update_config(config);
        self.find_apps();

        let icons_dir = self.icons_dir.clone();
        let paths = self
            .apps
            .iter()
            .filter_map(App::icon_path)
            .collect::<Vec<_>>();

        std::fs::create_dir_all(icons_dir)?;
        icon::extract_multiple(paths)?;

        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<ResultItem<'_>>>> {
        Ok(self
            .apps
            .iter()
            .filter_map(|app| app.fuzzy_match(query, matcher))
            .collect_non_empty())
    }

    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(app) = self.apps.iter().find(|app| app.id() == id) {
            app.execute(elevated)?;
        }
        Ok(())
    }

    fn show_item_in_dir(&self, id: &str) -> anyhow::Result<()> {
        if let Some(app) = self.apps.iter().find(|app| app.id() == id) {
            app.show_item_in_dir()?;
        }
        Ok(())
    }
}
