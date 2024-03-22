use crate::{
    config::Config,
    plugin::{Plugin, PluginStore},
};

mod app_launcher;
mod directory_indexer;
mod packaged_app_launcher;
mod shortcuts;
mod system_commands;

pub fn all(config: &Config) -> anyhow::Result<PluginStore> {
    let store = PluginStore::new();
    {
        let mut inner = store.lock();
        inner.add(app_launcher::Plugin::new(config)?);
        inner.add(packaged_app_launcher::Plugin::new(config)?);
        inner.add(directory_indexer::Plugin::new(config)?);
        inner.add(shortcuts::Plugin::new(config)?);
        inner.add(system_commands::Plugin::new(config)?);
    }
    Ok(store)
}
