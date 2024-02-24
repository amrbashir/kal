use std::{fmt::Display, process::Command, str::FromStr};

use crate::{
    common::{
        icon::{Defaults, Icon},
        SearchResultItem,
    },
    config::Config,
};
use serde::{Deserialize, Serialize};
use windows::Win32::System::Power::SetSuspendState;

#[derive(Clone, Copy)]
enum SystemCommand {
    Shutdown,
    Restart,
    SignOut,
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
            _ => return Err(SystemCommandParseError(s.to_string())),
        })
    }
}

impl SystemCommand {
    const fn all() -> [Self; 5] {
        [
            Self::Shutdown,
            Self::Restart,
            Self::SignOut,
            Self::Hibernate,
            Self::Sleep,
        ]
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Shutdown => Defaults::Shutdown.icon(),
            Self::Restart => Defaults::Restart.icon(),
            Self::SignOut => Defaults::SignOut.icon(),
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
            Self::Shutdown => &["/s", "/t", "5000"],
            Self::Restart => &["/r", "/t", "5000"],
            Self::SignOut => &["/l"],
            Self::Hibernate => &["/h"],
            Self::Sleep => unreachable!(),
        }
    }

    fn execute(&self) -> anyhow::Result<()> {
        match self {
            SystemCommand::Sleep => {
                unsafe { SetSuspendState(false, false, false) };
            }
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
    name: String,
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

impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> anyhow::Result<Box<Self>> {
        let name = "SystemCommands".to_string();
        let config = config.plugin_config::<PluginConfig>(&name);
        let results = SystemCommand::all().iter().map(|c| c.item(&name)).collect();

        Ok(Box::new(Self {
            name,
            enabled: config.enabled,
            results,
        }))
    }

    fn enabled(&self) -> bool {
        self.enabled
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn refresh(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(&self.name);
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
