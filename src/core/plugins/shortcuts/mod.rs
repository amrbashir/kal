use crate::{
    common::{
        icon::{Defaults, Icon},
        IntoSearchResultItem, SearchResultItem,
    },
    config::Config,
    utils::{self, IteratorExt},
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use url::Url;

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

    fn execute(&self, elevated: bool) -> anyhow::Result<()> {
        match &self.kind {
            ShortcutKind::Path { path } => utils::execute(path, elevated),
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
            utils::reveal_in_dir(path)?;
        }

        Ok(())
    }
}

impl IntoSearchResultItem for Shortcut {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<SearchResultItem> {
        matcher
            .fuzzy_match(&self.name, query)
            .or_else(|| {
                self.description
                    .as_ref()
                    .and_then(|description| matcher.fuzzy_match(description, query))
            })
            .map(|score| SearchResultItem {
                primary_text: self.name.as_str().into(),
                identifier: self.identifier.as_str().into(),
                secondary_text: self.description.as_deref().unwrap_or_default().into(),
                icon: self.icon(),
                needs_confirmation: self.needs_confirmation,
                score,
            })
    }
}

#[derive(Debug)]
pub struct Plugin {
    shortcuts: Vec<Shortcut>,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PluginConfig {
    shortcuts: Vec<Shortcut>,
}

impl Plugin {
    const NAME: &'static str = "Shortcuts";

    fn update_ids(&mut self) {
        self.shortcuts
            .iter_mut()
            .enumerate()
            .for_each(|(idx, shortcut)| {
                shortcut.identifier = if shortcut.identifier.is_empty() {
                    format!("{}:{idx}", Self::NAME)
                } else {
                    format!("{}:{}", Self::NAME, shortcut.identifier)
                };
            });
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config, _: &Path) -> anyhow::Result<Self> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);

        Ok(Self {
            shortcuts: config.shortcuts,
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(self.name());

        self.shortcuts = config.shortcuts;
        self.update_ids();

        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
        Ok(self
            .shortcuts
            .iter()
            .filter_map(|shortcut| shortcut.fuzzy_match(query, matcher))
            .collect_non_empty())
    }

    fn execute(&mut self, identifier: &str, elevated: bool) -> anyhow::Result<()> {
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
