use std::{path::Path, process::Command};

use crate::{
    common::{
        icon::{BuiltinIcon, Icon},
        IntoSearchResultItem, SearchResultItem,
    },
    config::Config,
    utils::IteratorExt,
};
use fuzzy_matcher::{skim::SkimMatcherV2, FuzzyMatcher};
use windows::Win32::System::{Power::SetSuspendState, Shutdown::LockWorkStation};

#[derive(Clone, Copy, Debug)]
enum SystemCommand {
    Shutdown,
    Restart,
    SignOut,
    Lock,
    Hibernate,
    Sleep,
}

impl AsRef<str> for SystemCommand {
    fn as_ref(&self) -> &str {
        self.str()
    }
}

impl SystemCommand {
    const fn all() -> [Self; 6] {
        [
            Self::Shutdown,
            Self::Restart,
            Self::SignOut,
            Self::Hibernate,
            Self::Sleep,
            Self::Lock,
        ]
    }

    const fn str(&self) -> &str {
        match self {
            Self::Shutdown => "Shutdown",
            Self::Restart => "Restart",
            Self::SignOut => "SignOut",
            Self::Lock => "Lock",
            Self::Hibernate => "Hibernate",
            Self::Sleep => "Sleep",
        }
    }

    const fn description(&self) -> &str {
        match self {
            Self::Shutdown => "Shut down computer",
            Self::Restart => "Restart computer",
            Self::SignOut => "Sign out current user",
            Self::Lock => "Lock current user profile",
            Self::Hibernate => "Put computer to hibernation",
            Self::Sleep => "Put computer to sleep",
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Shutdown => BuiltinIcon::Shutdown.icon(),
            Self::Restart => BuiltinIcon::Restart.icon(),
            Self::SignOut => BuiltinIcon::SignOut.icon(),
            Self::Lock => BuiltinIcon::Lock.icon(),
            Self::Hibernate => BuiltinIcon::Hibernate.icon(),
            Self::Sleep => BuiltinIcon::Sleep.icon(),
        }
    }

    const fn id(&self) -> &str {
        match self {
            Self::Shutdown => "SystemCommands:Shutdown",
            Self::Restart => "SystemCommands:Restart",
            Self::SignOut => "SystemCommands:SignOut",
            Self::Lock => "SystemCommands:Lock",
            Self::Hibernate => "SystemCommands:Hibernate",
            Self::Sleep => "SystemCommands:Sleep",
        }
    }

    const fn shutdown_bin_args(&self) -> &[&str] {
        match self {
            Self::Shutdown => &["/s", "/hybrid", "/t", "0"],
            Self::Restart => &["/r", "/t", "0"],
            Self::SignOut => &["/l"],
            Self::Hibernate => &["/h"],
            _ => unreachable!(),
        }
    }

    fn execute(&self) -> anyhow::Result<()> {
        match self {
            SystemCommand::Sleep => unsafe {
                SetSuspendState(false, true, true);
            },
            SystemCommand::Lock => unsafe { LockWorkStation()? },
            _ => {
                let shutdown_bin = std::env::var("SYSTEMROOT").map_or_else(
                    |_| "shutdown.exe".to_string(),
                    |p| format!("{p}\\System32\\shutdown.exe"),
                );
                Command::new(shutdown_bin)
                    .args(self.shutdown_bin_args())
                    .spawn()?;
            }
        }

        Ok(())
    }
}

impl IntoSearchResultItem for SystemCommand {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<SearchResultItem> {
        matcher.fuzzy_match(self.as_ref(), query).map(|score| {
            let primary_text = self.as_ref().into();
            let icon = self.icon();
            let id = self.id().into();
            let secondary_text = self.description().into();
            SearchResultItem {
                primary_text,
                secondary_text,
                icon,
                needs_confirmation: true,
                id,
                score,
            }
        })
    }
}

#[derive(Debug)]
pub struct Plugin {
    commands: [SystemCommand; 6],
}

impl Plugin {
    const NAME: &'static str = "SystemCommands";
}

impl crate::plugin::Plugin for Plugin {
    fn new(_config: &Config, _: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            commands: SystemCommand::all(),
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn refresh(&mut self, _config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    fn results(
        &mut self,
        query: &str,
        matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
        Ok(self
            .commands
            .iter()
            .filter_map(|c| c.fuzzy_match(query, matcher))
            .collect_non_empty())
    }

    fn execute(&mut self, id: &str, _elevated: bool) -> anyhow::Result<()> {
        if let Some(command) = self.commands.iter().find(|command| command.id() == id) {
            command.execute()?;
        }
        Ok(())
    }
}
