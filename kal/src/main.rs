use std::path::PathBuf;

use anyhow::Context;
use tracing_subscriber::layer::SubscriberExt;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod config;
#[cfg(not(debug_assertions))]
mod embedded_assets;
mod icon;
mod ipc;
mod main_window;
mod plugin;
mod plugin_store;
mod plugins;
mod result_item;
mod utils;
mod webview_window;

fn run(data_dir: PathBuf) -> anyhow::Result<()> {
    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();

    let mut app = crate::app::App::new(data_dir, proxy)?;

    event_loop.run_app(&mut app).map_err(Into::into)
}

fn main() -> anyhow::Result<()> {
    // Use two threads for async
    std::env::set_var("SMOL_THREADS", "2");

    let data_dir = dirs::data_local_dir()
        .context("Failed to get $data_local_dir path")?
        .join("kal");

    #[cfg(not(debug_assertions))]
    let (layer, _guard) = {
        let appender = tracing_appender::rolling::daily(&data_dir, "kal.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(appender);
        let layer = tracing_subscriber::fmt::Layer::default()
            // disable ansi coloring in log file
            .with_ansi(false)
            .with_writer(non_blocking);

        (layer, _guard)
    };

    let env_filter = tracing_subscriber::EnvFilter::from_env("KAL_LOG");

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_max_level(tracing::Level::TRACE)
        .finish()
        .with(env_filter);

    #[cfg(not(debug_assertions))]
    let subscriber = subscriber.with(layer);

    tracing::subscriber::set_global_default(subscriber)?;

    tracing::debug!("Logger initialized");

    run(data_dir).inspect_err(|e| tracing::error!("{e}"))
}
