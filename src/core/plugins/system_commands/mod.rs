use std::{fmt::Display, process::Command, str::FromStr};

use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
};
use serde::{Deserialize, Serialize};
use windows::Win32::System::{Power::SetSuspendState, Shutdown::LockWorkStation};

#[derive(Clone, Copy)]
enum SystemCommand {
    Shutdown,
    Restart,
    SignOut,
    Lock,
    Hibernate,
    Sleep,
}

impl Display for SystemCommand {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                Self::Shutdown => "Shutdown",
                Self::Restart => "Restart",
                Self::SignOut => "SignOut",
                Self::Lock => "Lock",
                Self::Hibernate => "Hibernate",
                Self::Sleep => "Sleep",
            }
        )
    }
}

#[derive(Debug)]
struct SystemCommandParseError(String);

impl Display for SystemCommandParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unkown system command: {}", self.0)
    }
}

impl std::error::Error for SystemCommandParseError {}

impl FromStr for SystemCommand {
    type Err = SystemCommandParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "Shutdown" => Self::Shutdown,
            "Restart" => Self::Restart,
            "SignOut" => Self::SignOut,
            "Hibernate" => Self::Hibernate,
            "Sleep" => Self::Sleep,
            "Lock" => Self::Lock,
            _ => return Err(SystemCommandParseError(s.to_string())),
        })
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

    fn item(&self, plugin_name: &str) -> SearchResultItem {
        let plugin_name = plugin_name.to_string();
        let icon = self.icon();
        let execution_args = self.to_string().into();
        match self {
            Self::Shutdown => SearchResultItem {
                primary_text: "Shutdown".into(),
                secondary_text: "Shut down computer".into(),
                execution_args,
                icon,
                plugin_name,
                needs_confirmation: true,
            },
            Self::Restart => SearchResultItem {
                primary_text: "Restart".into(),
                secondary_text: "Restart computer".into(),
                execution_args,
                icon,
                plugin_name,
                needs_confirmation: true,
            },
            Self::SignOut => SearchResultItem {
                primary_text: "Sign Out".into(),
                secondary_text: "Sign out current user".into(),
                execution_args,
                icon,
                plugin_name,
                needs_confirmation: true,
            },
            Self::Lock => SearchResultItem {
                primary_text: "Lock".into(),
                secondary_text: "Lock current user profile".into(),
                execution_args,
                icon,
                plugin_name,
                needs_confirmation: true,
            },
            Self::Hibernate => SearchResultItem {
                primary_text: "Hibernate".into(),
                secondary_text: "Put computer to hibernation".into(),
                execution_args,
                icon,
                plugin_name,
                needs_confirmation: true,
            },
            Self::Sleep => SearchResultItem {
                primary_text: "Sleep".into(),
                secondary_text: "Put computer to sleep".into(),
                execution_args,
                icon,
                plugin_name,
                needs_confirmation: true,
            },
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
    enabled: bool,
    results: Vec<SearchResultItem>,
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

impl Plugin {
    const NAME: &'static str = "SystemCommands";

    fn name(&self) -> &str {
        Self::NAME
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Box<Self>> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        let results = SystemCommand::all()
            .iter()
            .map(|c| c.item(Self::NAME))
            .collect();

        Ok(Box::new(Self {
            enabled: config.enabled,
            results,
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
        Ok(())
    }

    fn results(&self, _query: &str) -> anyhow::Result<&[SearchResultItem]> {
        Ok(&self.results)
    }

    fn execute(&self, item: &SearchResultItem, _elevated: bool) -> anyhow::Result<()> {
        let command: SystemCommand = item.str()?.parse()?;
        command.execute()
    }
}
