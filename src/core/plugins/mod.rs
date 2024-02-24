use crate::{config::Config, plugin::Plugin};

mod app_launcher;
mod directory_indexer;
mod shortcuts;
mod system_commands;

pub fn all(config: &Config) -> anyhow::Result<Vec<Box<dyn Plugin + Send + 'static>>> {
    Ok(vec![
        app_launcher::Plugin::new(config)?,
        directory_indexer::Plugin::new(config)?,
        shortcuts::Plugin::new(config)?,
        system_commands::Plugin::new(config)?,
    ])
}
