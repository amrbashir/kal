use std::process::Command;

use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
};
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use windows::Win32::System::{Power::SetSuspendState, Shutdown::LockWorkStation};

const PLUGIN_NAME: &str = "SystemCommands";

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

impl<'a> From<&'a SystemCommand> for SearchResultItem<'a> {
    fn from(command: &'a SystemCommand) -> Self {
        let primary_text = command.as_ref().into();
        let icon = command.icon();
        let identifier = command.identifier().into();
        let secondary_text = command.description().into();
        SearchResultItem {
            primary_text,
            secondary_text,
            icon,
            needs_confirmation: true,
            identifier,
        }
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
            Self::Shutdown => Defaults::Shutdown.icon(),
            Self::Restart => Defaults::Restart.icon(),
            Self::SignOut => Defaults::SignOut.icon(),
            Self::Lock => Defaults::Lock.icon(),
            Self::Hibernate => Defaults::Hibernate.icon(),
            Self::Sleep => Defaults::Sleep.icon(),
        }
    }

    const fn identifier(&self) -> &str {
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

#[derive(Debug)]
pub struct Plugin {
    commands: [SystemCommand; 6],
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    enabled: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(_config: &Config) -> anyhow::Result<Self> {
        Ok(Self {
            commands: SystemCommand::all(),
        })
    }

    fn name(&self) -> &str {
        PLUGIN_NAME
    }

    fn refresh(&mut self, _config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    fn results(
        &self,
        query: &str,
        matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<Vec<SearchResultItem<'_>>> {
        let filtered = self
            .commands
            .iter()
            .filter(|c| matcher.fuzzy_match(c.as_ref(), query).is_some())
            .map(Into::into)
            .collect();

        Ok(filtered)
    }

    fn execute(&self, identifier: &str, _elevated: bool) -> anyhow::Result<()> {
        if let Some(command) = self
            .commands
            .iter()
            .find(|command| command.identifier() == identifier)
        {
            command.execute()?;
        }
        Ok(())
    }
}
