use kal_config::Config;
use kal_plugin::{Action, BuiltinIcon, Icon, IntoResultItem, PluginQueryOutput, ResultItem};
use kal_utils::IteratorExt;
use strum::AsRefStr;

#[derive(Debug)]
pub struct Plugin {
    commands: [SystemCommand; 6],
}

impl Plugin {
    const NAME: &'static str = "SystemCommands";

    fn all(&self) -> Option<Vec<ResultItem>> {
        self.commands
            .iter()
            .map(|workflow| workflow.item(0))
            .collect_non_empty()
    }

    fn all_for_query(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<Vec<ResultItem>> {
        self.commands
            .iter()
            .filter_map(|workflow| workflow.fuzzy_match(query, matcher))
            .collect_non_empty()
    }
}

#[async_trait::async_trait]
impl kal_plugin::Plugin for Plugin {
    fn new(_config: &Config) -> Self {
        Self {
            commands: SystemCommand::all(),
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_plugin_config(&self) -> kal_config::PluginConfig {
        kal_config::PluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: Some("!".into()),
            inner: None,
        }
    }

    async fn reload(&mut self, _config: &Config) -> anyhow::Result<()> {
        Ok(())
    }

    async fn query(
        &mut self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        Ok(self.all_for_query(query, matcher).into())
    }

    async fn query_direct(
        &mut self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        if query.is_empty() {
            Ok(self.all().into())
        } else {
            Ok(self.all_for_query(query, matcher).into())
        }
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
            Self::Shutdown => BuiltinIcon::Shutdown.into(),
            Self::Restart => BuiltinIcon::Restart.into(),
            Self::SignOut => BuiltinIcon::SignOut.into(),
            Self::Lock => BuiltinIcon::Lock.into(),
            Self::Hibernate => BuiltinIcon::Hibernate.into(),
            Self::Sleep => BuiltinIcon::Sleep.into(),
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

        if res != rfd::MessageDialogResult::Yes {
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

    fn item(&self, score: u16) -> ResultItem {
        let system_command = *self;
        ResultItem {
            id: self.id().into(),
            icon: self.icon(),
            primary_text: self.as_ref().into(),
            secondary_text: self.description().into(),
            tooltip: None,
            actions: vec![Action::primary(move |_| system_command.execute())],
            score,
        }
    }
}

impl IntoResultItem for SystemCommand {
    fn fuzzy_match(
        &self,
        query: &str,
        matcher: &mut kal_plugin::FuzzyMatcher,
    ) -> Option<ResultItem> {
        matcher
            .fuzzy_match(self.as_ref(), query)
            .map(|score| self.item(score))
    }
}
