use serde::{Deserialize, Serialize};

use crate::config::{Config, GenericPluginConfig};
use crate::icon::BuiltInIcon;
use crate::result_item::ResultItem;
use crate::utils;

#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug)]
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
}

#[derive(Debug)]
pub struct Plugin {
    shell: Shell,
    no_exit: bool,
    last_command: String,
}

#[derive(Serialize, Deserialize, Debug, Default)]
struct PluginConfig {
    shell: Option<Shell>,
    no_exit: Option<bool>,
}

impl Plugin {
    const NAME: &'static str = "Shell";
    const ID: &'static str = "Shell-99999-item";

    fn item(&self) -> ResultItem<'_> {
        ResultItem {
            primary_text: self.last_command.as_str().into(),
            secondary_text: "Run command through shell".into(),
            needs_confirmation: false,
            id: Self::ID.into(),
            icon: BuiltInIcon::Shell.icon(),
            score: 0,
        }
    }
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
            last_command: String::new(),
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

    fn results(
        &mut self,
        query: &str,
        _matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<crate::result_item::ResultItem<'_>>>> {
        self.last_command = query.to_string();
        Ok(Some(vec![self.item()]))
    }

    fn execute(&mut self, id: &str, elevated: bool) -> anyhow::Result<()> {
        if id == Self::ID {
            let exe = self.shell.exe();
            let args = self.shell.args(self.no_exit);
            let args = format!("{args} {}", self.last_command);
            utils::execute_with_args(exe, args, elevated)?;
        }

        Ok(())
    }
}
