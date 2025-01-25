use std::path::Path;

use crate::config::Config;
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

pub fn all(config: &Config, data_dir: &Path) -> PluginStore {
    PluginStore::new(vec![
        app_launcher::Plugin::new(config, data_dir).into(),
        calculator::Plugin::new(config, data_dir).into(),
        directory_indexer::Plugin::new(config, data_dir).into(),
        everything::Plugin::new(config, data_dir).into(),
        shell::Plugin::new(config, data_dir).into(),
        system_commands::Plugin::new(config, data_dir).into(),
        workflows::Plugin::new(config, data_dir).into(),
        vscode_workspaces::Plugin::new(config, data_dir).into(),
    ])
}
