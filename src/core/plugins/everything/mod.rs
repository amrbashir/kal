use crate::{
    common::{icon::Icon, IntoSearchResultItem, SearchResultItem},
    config::{Config, GenericPluginConfig},
    utils::{self, PathExt},
};
use fuzzy_matcher::skim::SkimMatcherV2;
use serde::{Deserialize, Serialize};
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
};

#[derive(Debug)]
struct EverythingEntry {
    name: OsString,
    path: PathBuf,
    icon: PathBuf,
    is_dir: bool,
    identifier: String,
}

impl EverythingEntry {
    fn new(path: &str, icons_dir: &Path) -> Self {
        let path = PathBuf::from(path);
        let name = path.file_name().unwrap_or_default().to_os_string();
        let is_dir = path.metadata().map(|m| m.is_dir()).unwrap_or_default();
        let identifier = format!("{}:{}", Plugin::NAME, name.to_string_lossy());
        let icon = icons_dir.join(&name).with_extra_extension("png");
        let _ = utils::extract_icon_cached(&path, &icon);
        Self {
            name,
            path,
            icon,
            identifier,
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

    fn reveal_in_dir(&self) -> anyhow::Result<()> {
        utils::reveal_in_dir(&self.path)
    }
}

impl IntoSearchResultItem for EverythingEntry {
    fn fuzzy_match(&self, _query: &str, _matcher: &SkimMatcherV2) -> Option<SearchResultItem> {
        Some(SearchResultItem {
            primary_text: self.name.to_string_lossy(),
            secondary_text: self.path.to_string_lossy(),
            icon: Icon::path(self.icon.to_string_lossy()),
            needs_confirmation: false,
            identifier: self.identifier.as_str().into(),
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

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.es = config.es.unwrap_or_else(|| PathBuf::from("es"));
        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
        let output = std::process::Command::new(&self.es)
            .arg(query)
            .args(["-n", "24"]) // TODO: pull from config
            .output()?;

        if !output.status.success() {
            return Ok(None);
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

    fn execute(&mut self, identifier: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.identifier == identifier) {
            entry.execute(elevated)?;
        }
        Ok(())
    }

    fn reveal_in_dir(&self, identifier: &str) -> anyhow::Result<()> {
        if let Some(entry) = self.entries.iter().find(|e| e.identifier == identifier) {
            entry.reveal_in_dir()?;
        }
        Ok(())
    }
}
