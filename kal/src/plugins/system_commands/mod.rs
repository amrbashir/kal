use std::path::Path;

use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use strum::AsRefStr;

use crate::config::Config;
use crate::icon::{BuiltInIcon, Icon};
use crate::result_item::{Action, IntoResultItem, QueryReturn, ResultItem};
use crate::utils::IteratorExt;

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

    fn reload(&mut self, _config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    fn query(
        &mut self,
        query: &str,
        matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<QueryReturn> {
        Ok(self
            .commands
            .iter()
            .filter_map(|c| c.fuzzy_match(query, matcher))
            .collect_non_empty::<Vec<_>>()
            .into())
    }
}

#[derive(Clone, Copy, Debug, AsRefStr)]
enum SystemCommand {
    Shutdown,
    Restart,
    SignOut,
    Lock,
    Hibernate,
    Sleep,
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

    const fn description(&self) -> &str {
        match self {
            Self::Shutdown => "Shutdown computer",
            Self::Restart => "Restart computer",
            Self::SignOut => "Sign out of computer",
            Self::Lock => "Lock computer",
            Self::Hibernate => "Hibernate computer",
            Self::Sleep => "Put computer to sleep",
        }
    }

    const fn confirm_message(&self) -> &str {
        match self {
            Self::Shutdown => "shut down this computer",
            Self::Restart => "restart this computer",
            Self::SignOut => "sign out of this computer",
            Self::Lock => "lock this computer",
            Self::Hibernate => "put this computer into hibernation",
            Self::Sleep => "put this computer to sleep",
        }
    }

    fn icon(&self) -> Icon {
        match self {
            Self::Shutdown => BuiltInIcon::Shutdown.icon(),
            Self::Restart => BuiltInIcon::Restart.icon(),
            Self::SignOut => BuiltInIcon::SignOut.icon(),
            Self::Lock => BuiltInIcon::Lock.icon(),
            Self::Hibernate => BuiltInIcon::Hibernate.icon(),
            Self::Sleep => BuiltInIcon::Sleep.icon(),
        }
    }

    const fn id(&self) -> &str {
        match self {
            Self::Shutdown => "SystemCommand:Shutdown",
            Self::Restart => "SystemCommand:Restart",
            Self::SignOut => "SystemCommand:SignOut",
            Self::Lock => "SystemCommand:Lock",
            Self::Hibernate => "SystemCommand:Hibernate",
            Self::Sleep => "SystemCommand:Sleep",
        }
    }

    #[cfg(windows)]
    const fn shutdown_bin_args(&self) -> &[&str] {
        match self {
            Self::Shutdown => &["/s", "/hybrid", "/t", "0"],
            Self::Restart => &["/r", "/t", "0"],
            Self::SignOut => &["/l"],
            Self::Hibernate => &["/h"],
            _ => unreachable!(),
        }
    }

    #[cfg(windows)]
    fn execute(&self) -> anyhow::Result<()> {
        use std::process::Command;

        use windows::Win32::System::Power::SetSuspendState;
        use windows::Win32::System::Shutdown::LockWorkStation;

        let res = rfd::MessageDialog::new()
            .set_title("Please confirm")
            .set_description(format!(
                "You are about to {}, are you sure?",
                self.confirm_message()
            ))
            .set_level(rfd::MessageLevel::Warning)
            .set_buttons(rfd::MessageButtons::YesNo)
            .show();

        let confirm = res == rfd::MessageDialogResult::Yes;

        if !confirm {
            return Ok(());
        }

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

    #[cfg(not(windows))]
    const fn shutdown_bin_args(&self) -> &[&str] {
        unimplemented!()
    }

    #[cfg(not(windows))]
    fn execute(&self) -> anyhow::Result<()> {
        unimplemented!()
    }
}

impl IntoResultItem for SystemCommand {
    fn fuzzy_match(&self, query: &str, matcher: &SkimMatcherV2) -> Option<ResultItem> {
        matcher.fuzzy_match(self.as_ref(), query).map(|score| {
            let system_command = *self;
            ResultItem {
                id: self.id().into(),
                icon: self.icon(),
                primary_text: self.as_ref().into(),
                secondary_text: self.description().into(),
                actions: vec![Action::primary(move |_| system_command.execute())],
                score,
            }
        })
    }
}
