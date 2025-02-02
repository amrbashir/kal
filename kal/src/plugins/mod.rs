use kal_config::Config;
use kal_plugin::Plugin;

use crate::plugin_manager::PluginManager;

mod app_launcher;
mod calculator;
mod directory_indexer;
#[cfg(windows)]
mod everything;
mod shell;
mod system_commands;
mod vscode_workspaces;
mod workflows;

pub fn all(config: &Config) -> PluginManager {
    PluginManager::new(vec![
        app_launcher::Plugin::new(config).into(),
        calculator::Plugin::new(config).into(),
        directory_indexer::Plugin::new(config).into(),
        #[cfg(windows)]
        everything::Plugin::new(config).into(),
        shell::Plugin::new(config).into(),
        system_commands::Plugin::new(config).into(),
        workflows::Plugin::new(config).into(),
        vscode_workspaces::Plugin::new(config).into(),
    ])
}
