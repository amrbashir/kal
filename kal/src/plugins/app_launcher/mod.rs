use std::path::Path;
use std::sync::{Arc, Mutex};

use kal_config::Config;
use notify::RecommendedWatcher;
use notify_debouncer_mini::Debouncer;
use serde::{Deserialize, Serialize};
use windows::ApplicationModel::PackageCatalog;

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
    apps: Arc<Mutex<Vec<App>>>,
    programs_watcher: Option<Debouncer<RecommendedWatcher>>,
    #[cfg(windows)]
    package_catalog: Option<PackageCatalog>,
}

#[cfg(windows)]
unsafe impl Send for Plugin {}
#[cfg(windows)]
unsafe impl Sync for Plugin {}

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
        let config = config.plugin_config_inner::<PluginConfig>(Self::NAME);
        self.paths = config.paths;
        self.extensions = config.extensions;
        self.include_packaged_apps = config.include_packaged_apps;
    }

    async fn find_apps(&mut self) {
        *self.apps.lock().unwrap() =
            program::find_all_in_paths(&self.paths, &self.extensions).await;

        #[cfg(windows)]
        if self.include_packaged_apps {
            if let Ok(packaged_apps) = packaged_app::find_all() {
                self.apps
                    .lock()
                    .unwrap()
                    .extend(packaged_apps.map(App::Packaged));
            }
        }
    }
}

#[async_trait::async_trait]
impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> Self {
        let config = config.plugin_config_inner::<PluginConfig>(Self::NAME);

        Self {
            paths: config.paths,
            extensions: config.extensions,
            include_packaged_apps: config.include_packaged_apps,
            apps: Default::default(),
            programs_watcher: None,
            package_catalog: None,
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_plugin_config(&self) -> kal_config::PluginConfig {
        kal_config::PluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: Some(".".into()),
            inner: toml::Table::try_from(PluginConfig::default()).ok(),
        }
    }

    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        self.update_config(config);
        self.find_apps().await;
        self.watch_programs()?;

        #[cfg(windows)]
        if self.package_catalog.is_none() {
            self.watch_packaged_apps()?;
        }

        Ok(())
    }

    async fn query(
        &mut self,
        query: &str,
        matcher: &mut crate::fuzzy_matcher::Matcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        if query.is_empty() {
            return Ok(PluginQueryOutput::None);
        }

        Ok(self
            .apps
            .lock()
            .unwrap()
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
    pub fn name(&self) -> &str {
        match self {
            App::Program(program) => program.name.to_str().unwrap_or_default(),
            App::Packaged(packaged_app) => &packaged_app.name,
        }
    }

    pub fn path(&self) -> Option<&Path> {
        match self {
            App::Program(program) => Some(&program.path),
            App::Packaged(_) => None,
        }
    }
}

impl IntoResultItem for App {
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut crate::fuzzy_matcher::Matcher,
    ) -> Option<ResultItem> {
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
        "%PUBLIC%\\Desktop".to_string(),
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
