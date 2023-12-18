use std::{fmt::Display, path::PathBuf};

use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
    plugin::Plugin,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[cfg(windows)]
#[path = "windows.rs"]
mod platform;

#[derive(Debug)]
pub struct ShortcutsPlugin {
    name: String,
    enabled: bool,
    shortcuts: Vec<ShortCut>,
    cached_shortcuts: Vec<SearchResultItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShortcutsPluginConfig {
    pub enabled: bool,
    pub shortcuts: Vec<ShortCut>,
}

impl Default for ShortcutsPluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcuts: Default::default(),
        }
    }
}

impl Plugin for ShortcutsPlugin {
    fn new(config: &Config) -> anyhow::Result<Box<Self>> {
        let name = "Shortcuts".to_string();
        let config = config.plugin_config::<ShortcutsPluginConfig>(&name);

        Ok(Box::new(Self {
            name,
            enabled: config.enabled,
            shortcuts: config.shortcuts,
            cached_shortcuts: Vec::new(),
        }))
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn refresh(&mut self, config: &Config) {
        let config = config.plugin_config::<ShortcutsPluginConfig>(&self.name);
        self.enabled = config.enabled;
        self.shortcuts = config.shortcuts;

        self.cached_shortcuts = self
            .shortcuts
            .iter()
            .enumerate()
            .map(|(i, shortcut)| SearchResultItem {
                primary_text: shortcut.name.clone(),
                secondary_text: shortcut.to_string(),
                plugin_name: self.name.clone(),
                execution_args: serde_json::Value::Number(serde_json::Number::from(i)),
                icon: shortcut.icon(),
            })
            .collect();
    }

    fn results(&self, _query: &str) -> &[SearchResultItem] {
        &self.cached_shortcuts
    }

    fn execute(&self, item: &SearchResultItem, elevated: bool) {
        let index = item.execution_args.as_u64().unwrap();
        if let Some(shortcut) = self.shortcuts.get(index as usize) {
            match &shortcut.r#type {
                ShortCutType::Path { path } => platform::open_path(path),
                ShortCutType::Url { url } => platform::open_url(url),
                ShortCutType::Shell {
                    shell,
                    script,
                    working_directory,
                    hidden,
                } => platform::execute_in_shell(shell, script, working_directory, hidden, elevated),
            }
        }
    }

    fn open_location(&self, item: &SearchResultItem) {
        let index = item.execution_args.as_u64().unwrap();
        if let Some(shortcut) = self.shortcuts.get(index as usize) {
            if let ShortCutType::Path { path } = &shortcut.r#type {
                platform::open_location(path);
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ShortCut {
    pub name: String,
    pub description: Option<String>,
    #[serde(flatten)]
    pub r#type: ShortCutType,
    // TODO: add needs_confirmation
}

impl ShortCut {
    pub fn icon(&self) -> Icon {
        match &self.r#type {
            ShortCutType::Path { path } => {
                if path.is_file() {
                    Defaults::File.icon()
                } else {
                    Defaults::Folder.icon()
                }
            }
            ShortCutType::Url { .. } => Defaults::Url.icon(),
            ShortCutType::Shell { .. } => Defaults::Shell.icon(),
        }
    }
}

impl Display for ShortCut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(desc) = &self.description {
            write!(f, "{}", desc)
        } else {
            match &self.r#type {
                ShortCutType::Path { path } => {
                    write!(f, "[Path] {}", path.to_string_lossy())
                }
                ShortCutType::Url { url } => write!(f, "[URL] {}", url.clone()),
                ShortCutType::Shell { script, .. } => write!(f, "[Shell] {}", { script }),
            }
        }
    }
}

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ShortCutType {
    Path {
        path: PathBuf,
    },
    Url {
        url: Url,
    },
    Shell {
        shell: Option<String>,
        script: String,
        working_directory: Option<String>,
        hidden: Option<bool>,
    },
}
