use std::path::Path;

use crate::{
    config::Config,
    plugin::{Plugin, PluginStore},
};

mod app_launcher;
mod directory_indexer;
mod packaged_app_launcher;
mod shortcuts;
mod system_commands;

pub fn all(config: &Config, data_dir: &Path) -> anyhow::Result<PluginStore> {
    let store = PluginStore::new();
    {
        let mut inner = store.lock();
        inner.add(app_launcher::Plugin::new(config, data_dir)?);
        inner.add(packaged_app_launcher::Plugin::new(config, data_dir)?);
        inner.add(directory_indexer::Plugin::new(config, data_dir)?);
        inner.add(shortcuts::Plugin::new(config, data_dir)?);
        inner.add(system_commands::Plugin::new(config, data_dir)?);
    }
    Ok(store)
}
