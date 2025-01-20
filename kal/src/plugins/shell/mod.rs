use serde::{Deserialize, Serialize};

use crate::config::{Config, GenericPluginConfig};
use crate::icon::BuiltInIcon;
use crate::result_item::{Action, QueryReturn, ResultItem};
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

impl crate::plugin::Plugin for Plugin {
    fn new(config: &crate::config::Config, _data_dir: &std::path::Path) -> anyhow::Result<Self>
    where
        Self: Sized,
    {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        Ok(Self {
            shell: config.shell.unwrap_or_default(),
            no_exit: config.no_exit.unwrap_or_default(),
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(false),
            direct_activation_command: Some(">".into()),
        }
    }

    fn reload(&mut self, config: &Config) -> anyhow::Result<()> {
        let config = config.plugin_config::<PluginConfig>(Self::NAME);
        self.shell = config.shell.unwrap_or_default();
        self.no_exit = config.no_exit.unwrap_or_default();
        Ok(())
    }

    fn query(
        &mut self,
        query: &str,
        _matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<QueryReturn> {
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
            icon: BuiltInIcon::Shell.icon(),
            primary_text: command,
            secondary_text: Plugin::DESCRIPTION.into(),
            tooltip: None,
            actions: vec![
                Action::primary(move |item| {
                    let exe = shell.exe();
                    let args = shell.args(no_exit);
                    let args = format!("{args} {}", item.primary_text);
                    utils::execute_with_args(exe, args, false)
                }),
                Action::open_elevated(move |item| {
                    let exe = shell.exe();
                    let args = shell.args(no_exit);
                    let args = format!("{args} {}", item.primary_text);
                    utils::execute_with_args(exe, args, true)
                }),
            ],
            score: 0,
        }
    }
}
