use std::ffi::OsString;
use std::path::{Path, PathBuf};

use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};

use crate::config::{Config, GenericPluginConfig};
use crate::icon::{self, Icon};
use crate::result_item::{IntoResultItem, ResultItem};
use crate::utils::{self, PathExt};

#[derive(Debug)]
struct EverythingEntry {
    name: OsString,
    path: PathBuf,
    icon: PathBuf,
    is_dir: bool,
    id: String,
}

impl EverythingEntry {
    fn new(path: &str, icons_dir: &Path) -> Self {
        let path = PathBuf::from(path);
        let name = path.file_name().unwrap_or_default().to_os_string();
        let is_dir = path.metadata().map(|m| m.is_dir()).unwrap_or_default();
        let id = format!("{}:{}", Plugin::NAME, name.to_string_lossy());
        let icon = icons_dir.join(&name).with_extra_extension("png");
        let _ = icon::extract_cached(&path, &icon);
        Self {
            name,
            path,
            icon,
            id,
            is_dir,
        }
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        if self.is_dir {
            utils::open_dir(&self.path)
        } else {
            utils::execute(&self.path, elevated)
        }
    }

    fn show_item_in_dir(&self) -> anyhow::Result<()> {
        utils::reveal_in_dir(&self.path)
    }
}

impl IntoResultItem for EverythingEntry {
    fn fuzzy_match(&self, _query: &str, _matcher: &SkimMatcherV2) -> Option<ResultItem> {
        Some(ResultItem {
            primary_text: self.name.to_string_lossy(),
            secondary_text: self.path.to_string_lossy(),
            icon: Icon::path(self.icon.to_string_lossy()),
            needs_confirmation: false,
            id: self.id.as_str().into(),
            score: 200,
        })
    }
}

#[derive(Debug)]
pub struct Plugin {
    es: PathBuf,
    entries: Vec<EverythingEntry>,
    icons_dir: PathBuf,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PluginConfig {
    es: Option<PathBuf>,
}

impl Plugin {
    const NAME: &'static str = "Everything";
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config, data_dir: &Path) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        Ok(Self {
            es: config.es.unwrap_or_else(|| PathBuf::from("es")),
            entries: Vec::new(),
            icons_dir: data_dir.join("icons"),
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(false),
            direct_activation_command: Some(" ".into()),
        }
    }

    fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.es = config.es.unwrap_or_else(|| PathBuf::from("es"));
        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<ResultItem<'_>>>> {
        let output = std::process::Command::new(&self.es)
            .arg(query)
            .args(["-n", "24"]) // TODO: pull from config
            .output()?;

        match output.status.success() {
            true => {}
            false => {
                let stderr = std::str::from_utf8(&output.stderr).unwrap_or_default();
                tracing::error!("[Plugin][Everything]: {}", stderr);
                return Ok(None);
            }
        }

        let output = String::from_utf8_lossy(output.stdout.as_slice());

        self.entries = output
            .lines()
            .map(|e| EverythingEntry::new(e, &self.icons_dir))
            .collect::<Vec<_>>();

        Ok(self
            .entries
            .iter()
            .map(|e| e.fuzzy_match(query, matcher))
            .collect())
    }

    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
            entry.execute(elevated)?;
        }
        Ok(())
    }

    fn show_item_in_dir(&self, id: &str) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.id == id) {
            entry.show_item_in_dir()?;
        }
        Ok(())
    }
}
