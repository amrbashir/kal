use kal_config::Config;
use serde::{Deserialize, Serialize};

use crate::icon::BuiltinIcon;
use crate::plugin::PluginQueryOutput;
use crate::result_item::{Action, ResultItem};
use crate::utils;

#[derive(Debug)]
pub struct Plugin {
    shell: Shell,
    no_exit: bool,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PluginConfig {
    shell: Option<Shell>,
    no_exit: Option<bool>,
}

impl Plugin {
    const NAME: &str = "Shell";
    const ID: &str = "Shell";
    const DESCRIPTION: &str = "Shell: execute command through command shell";
}

#[async_trait::async_trait]
impl crate::plugin::Plugin for Plugin {
    fn new(config: &Config) -> Self {
        let config = config.plugin_config_inner::<PluginConfig>(Self::NAME);
        Self {
            shell: config.shell.unwrap_or_default(),
            no_exit: config.no_exit.unwrap_or_default(),
        }
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_plugin_config(&self) -> kal_config::PluginConfig {
        kal_config::PluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(false),
            direct_activation_command: Some(">".into()),
            inner: None,
        }
    }

    async fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config_inner::<PluginConfig>(Self::NAME);
        self.shell = config.shell.unwrap_or_default();
        self.no_exit = config.no_exit.unwrap_or_default();
        Ok(())
    }

    async fn query_direct(
        &mut self,
        query: &str,
        _matcher: &mut crate::fuzzy_matcher::Matcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        Ok(self.shell.item(query.to_string(), self.no_exit).into())
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug)]
#[allow(clippy::enum_variant_names)]
enum Shell {
    #[default]
    PowerShell7,
    PowerShell,
    CommandPrompt,
}

impl Shell {
    fn exe(&self) -> &str {
        match self {
            Shell::PowerShell7 => "pwsh.exe",
            Shell::PowerShell => "powershell.exe",
            Shell::CommandPrompt => "cmd.exe",
        }
    }

    fn args(&self, no_exit: bool) -> &str {
        match (self, no_exit) {
            (Shell::PowerShell7, false) => "-Command",
            (Shell::PowerShell7, true) => "-NoExit -Command",
            (Shell::PowerShell, false) => "-Command",
            (Shell::PowerShell, true) => "-NoExit -Command",
            (Shell::CommandPrompt, false) => "/C",
            (Shell::CommandPrompt, true) => "/K",
        }
    }

    fn item(&self, command: String, no_exit: bool) -> ResultItem {
        let shell = *self;

        ResultItem {
            id: Plugin::ID.into(),
            icon: BuiltinIcon::Shell.into(),
            primary_text: command,
            secondary_text: Plugin::DESCRIPTION.into(),
            tooltip: None,
            actions: vec![
                Action::primary(move |item| {
                    let exe = shell.exe();
                    let args = shell.args(no_exit);
                    let args = format!("{args} {}", item.primary_text);
                    utils::execute_with_args(exe, args, false, false)
                }),
                Action::open_elevated(move |item| {
                    let exe = shell.exe();
                    let args = shell.args(no_exit);
                    let args = format!("{args} {}", item.primary_text);
                    utils::execute_with_args(exe, args, true, false)
                }),
            ],
            score: 0,
        }
    }
}
