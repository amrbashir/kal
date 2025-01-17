use std::path::Path;

use crate::config::Config;
use crate::plugin::{Plugin, PluginStore};

mod app_launcher;
mod calculator;
mod directory_indexer;
mod everything;
mod system_commands;
mod workflows;

pub fn all(config: &Config, data_dir: &Path) -> anyhow::Result<PluginStore> {
    let store = PluginStore::new(vec![
        app_launcher::Plugin::new(config, data_dir)?.into(),
        directory_indexer::Plugin::new(config, data_dir)?.into(),
        workflows::Plugin::new(config, data_dir)?.into(),
        system_commands::Plugin::new(config, data_dir)?.into(),
        calculator::Plugin::new(config, data_dir)?.into(),
        everything::Plugin::new(config, data_dir)?.into(),
    ]);
    Ok(store)
}
