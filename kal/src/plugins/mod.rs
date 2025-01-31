use kal_config::Config;

use crate::plugin::Plugin;
use crate::plugin_store::PluginStore;

mod app_launcher;
mod calculator;
mod directory_indexer;
mod everything;
mod shell;
mod system_commands;
mod vscode_workspaces;
mod workflows;

pub fn all(config: &Config) -> PluginStore {
    PluginStore::new(vec![
        app_launcher::Plugin::new(config).into(),
        calculator::Plugin::new(config).into(),
        directory_indexer::Plugin::new(config).into(),
        everything::Plugin::new(config).into(),
        shell::Plugin::new(config).into(),
        system_commands::Plugin::new(config).into(),
        workflows::Plugin::new(config).into(),
        vscode_workspaces::Plugin::new(config).into(),
    ])
}
