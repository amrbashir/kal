use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
    utils,
};
use serde::{Deserialize, Serialize};
use std::{fmt::Display, path::PathBuf};
use url::Url;

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
pub enum ShortcutKind {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct Shortcut {
    pub name: String,
    pub description: Option<String>,
    #[serde(flatten)]
    pub kind: ShortcutKind,
    #[serde(default)]
    pub needs_confirmation: bool,
}

impl Shortcut {
    pub fn icon(&self) -> Icon {
        match &self.kind {
            ShortcutKind::Path { path } => {
                if path.is_file() {
                    // TODO: generate from file
                    Defaults::File.icon()
                } else {
                    Defaults::Folder.icon()
                }
            }
            ShortcutKind::Url { .. } => Defaults::Url.icon(),
            ShortcutKind::Shell { .. } => Defaults::Shell.icon(),
        }
    }
}

impl Display for Shortcut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(desc) = &self.description {
            write!(f, "{}", desc)
        } else {
            match &self.kind {
                ShortcutKind::Path { path } => {
                    write!(f, "[Path] {}", path.display())
                }
                ShortcutKind::Url { url } => write!(f, "[URL] {}", &url),
                ShortcutKind::Shell { script, .. } => write!(f, "[Shell] {}", { script }),
            }
        }
    }
}

#[derive(Debug)]
pub struct Plugin {
    enabled: bool,
    shortcuts: Vec<Shortcut>,
    cached_shortcuts: Vec<SearchResultItem>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PluginConfig {
    pub enabled: bool,
    pub shortcuts: Vec<Shortcut>,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            shortcuts: Default::default(),
        }
    }
}

impl Plugin {
    const NAME: &'static str = "Shortcuts";

    fn name(&self) -> &str {
        Self::NAME
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Box<Self>> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Box::new(Self {
            enabled: config.enabled,
            shortcuts: config.shortcuts,
            cached_shortcuts: Vec::new(),
        }))
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        self.name()
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(self.name());
        self.enabled = config.enabled;
        self.shortcuts = config.shortcuts;

        self.cached_shortcuts = self
            .shortcuts
            .iter()
            .enumerate()
            .map(|(i, shortcut)| SearchResultItem {
                primary_text: shortcut.name.clone(),
                secondary_text: shortcut.to_string(),
                plugin_name: self.name().to_string(),
                execution_args: serde_json::Value::Number(serde_json::Number::from(i)),
                icon: shortcut.icon(),
                needs_confirmation: shortcut.needs_confirmation,
            })
            .collect();

        Ok(())
    }

    fn results(&self, _query: &str) -> anyhow::Result<&[SearchResultItem]> {
        Ok(&self.cached_shortcuts)
    }

    fn execute(&self, item: &SearchResultItem, elevated: bool) -> anyhow::Result<()> {
        let index = item.index()?;
        if let Some(shortcut) = self.shortcuts.get(index as usize) {
            match &shortcut.kind {
                ShortcutKind::Path { path } => utils::open_path(path),
                ShortcutKind::Url { url } => utils::open_url(url),
                ShortcutKind::Shell {
                    shell,
                    script,
                    working_directory,
                    hidden,
                } => utils::execute_in_shell(shell, script, working_directory, hidden, elevated)?,
            }
        }
        Ok(())
    }

    fn open_location(&self, item: &SearchResultItem) -> anyhow::Result<()> {
        let index = item.index()?;
        if let Some(shortcut) = self.shortcuts.get(index as usize) {
            if let ShortcutKind::Path { path } = &shortcut.kind {
                utils::open_path(path);
            }
        }
        Ok(())
    }
}
