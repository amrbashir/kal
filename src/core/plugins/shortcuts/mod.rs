use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
    utils,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use url::Url;

const PLUGIN_NAME: &str = "Shortcuts";

#[derive(Deserialize, Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "type")]
enum ShortcutKind {
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
struct Shortcut {
    name: String,
    #[serde(default)]
    identifier: String,
    description: Option<String>,
    #[serde(flatten)]
    kind: ShortcutKind,
    #[serde(default)]
    needs_confirmation: bool,
}

impl<'a> From<&'a Shortcut> for SearchResultItem<'a> {
    fn from(shortcut: &'a Shortcut) -> Self {
        SearchResultItem {
            primary_text: shortcut.name.as_str().into(),
            identifier: shortcut.identifier.as_str().into(),
            secondary_text: shortcut.description.as_deref().unwrap_or_default().into(),
            icon: shortcut.icon(),
            needs_confirmation: shortcut.needs_confirmation,
        }
    }
}

impl Shortcut {
    fn icon(&self) -> Icon {
        match &self.kind {
            ShortcutKind::Path { path } => {
                if path.is_file() {
                    // TODO: generate from file
                    Defaults::File.icon()
                } else {
                    Defaults::Directory.icon()
                }
            }
            ShortcutKind::Url { .. } => Defaults::Url.icon(),
            ShortcutKind::Shell { .. } => Defaults::Shell.icon(),
        }
    }

    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> bool {
        matcher.fuzzy_match(&self.name, query).is_some()
            || self
                .description
                .as_ref()
                .map(|description| matcher.fuzzy_match(description, query).is_some())
                .unwrap_or(false)
    }

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        match &self.kind {
            ShortcutKind::Path { path } => utils::open_path(path),
            ShortcutKind::Url { url } => utils::open_url(url),
            ShortcutKind::Shell {
                shell,
                script,
                working_directory,
                hidden,
            } => utils::execute_in_shell(shell, script, working_directory, hidden, elevated),
        }
    }

    fn reveal_in_dir(&self) -> anyhow::Result<()> {
        if let ShortcutKind::Path { path } = &self.kind {
            utils::open_path(path)?;
        }

        Ok(())
    }
}

#[derive(Debug)]
pub struct Plugin {
    shortcuts: Vec<Shortcut>,
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    enabled: bool,
    shortcuts: Vec<Shortcut>,
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

    fn update_ids(&mut self) {
        self.shortcuts
            .iter_mut()
            .enumerate()
            .for_each(|(idx, shortcut)| {
                shortcut.identifier = if shortcut.identifier.is_empty() {
                    format!("{PLUGIN_NAME}:{idx}")
                } else {
                    format!("{PLUGIN_NAME}:{}", shortcut.identifier)
                };
            });
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Self {
            shortcuts: config.shortcuts,
        })
    }

    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(self.name());

        self.shortcuts = config.shortcuts;
        self.update_ids();

        Ok(())
    }

    fn results(
        &self,
        query: &str,
        matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<Vec<SearchResultItem<'_>>> {
        let filtered = self
            .shortcuts
            .iter()
            .filter(|shortcut| shortcut.fuzzy_match(query, matcher))
            .map(Into::into)
            .collect::<Vec<_>>();

        Ok(filtered)
    }

    fn execute(&self, identifier: &str, elevated: bool) -> anyhow::Result<()> {
        if let Some(shortcut) = self.shortcuts.iter().find(|s| s.identifier == identifier) {
            shortcut.execute(elevated)?;
        }
        Ok(())
    }

    fn reveal_in_dir(&self, identifier: &str) -> anyhow::Result<()> {
        if let Some(shortcut) = self.shortcuts.iter().find(|s| s.identifier == identifier) {
            shortcut.reveal_in_dir()?;
        }
        Ok(())
    }
}
