use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};

use crate::config::{Config, GenericPluginConfig};
use crate::icon;
use crate::plugin::PluginQueryOutput;
use crate::result_item::{IntoResultItem, ResultItem};
use crate::utils::IteratorExt;

#[cfg(windows)]
mod packaged_app;
mod program;

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

impl Plugin {
    const NAME: &'static str = "AppLauncher";

    fn update_config(&mut self, config: &Config) {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.paths = config.paths;
        self.extensions = config.extensions;
        self.include_packaged_apps = config.include_packaged_apps;
    }

    async fn find_apps(&mut self) {
        self.apps = program::find_all_in_paths(&self.paths, &self.extensions, &self.icons_dir)
            .await
            .into_iter()
            .map(App::Program)
            .collect();

        #[cfg(windows)]
        if self.include_packaged_apps {
            if let Ok(packaged_apps) = packaged_app::find_all() {
                self.apps.extend(packaged_apps.map(App::Packaged));
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config, data_dir: &Path) -> Self {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Self {
            paths: config.paths,
            extensions: config.extensions,
            include_packaged_apps: config.include_packaged_apps,
            icons_dir: data_dir.join("icons"),
            apps: Vec::new(),
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: Some(".".into()),
        }
    }

    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        self.update_config(config);
        self.find_apps().await;

        let icons_dir = self.icons_dir.clone();
        let paths = self
            .apps
            .iter()
            .filter_map(App::icon_path)
            .collect::<Vec<_>>();

        smol::fs::create_dir_all(icons_dir).await?;
        let _ = icon::extract_multiple(paths).inspect_err(|e| tracing::error!("{e}"));

        Ok(())
    }

    async fn query(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<PluginQueryOutput> {
        if query.is_empty() {
            return Ok(PluginQueryOutput::None);
        }

        Ok(self
            .apps
            .iter()
            .filter_map(|app| app.fuzzy_match(query, matcher))
            .collect_non_empty::<Vec<_>>()
            .into())
    }
}

#[derive(Debug)]
enum App {
    Program(program::Program),
    #[cfg(windows)]
    Packaged(packaged_app::PackagedApp),
}

impl App {
    fn icon_path(&self) -> Option<(PathBuf, PathBuf)> {
        match self {
            App::Program(program) => Some((program.path.clone(), program.icon.clone())),
            #[cfg(windows)]
            App::Packaged(_) => None,
        }
    }
}

impl IntoResultItem for App {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem> {
        match self {
            App::Program(program) => program.fuzzy_match(query, matcher),
            #[cfg(windows)]
            App::Packaged(packaged_app) => packaged_app.fuzzy_match(query, matcher),
        }
    }
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
