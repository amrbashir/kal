use std::path::PathBuf;

use anyhow::Context;
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
mod config;
mod embedded_assets;
mod icon;
mod ipc;
mod main_window;
mod plugin;
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
    let data_dir = dirs::data_local_dir()
        .context("Failed to get $data_local_dir path")?
        .join("kal");

    let appender = tracing_appender::rolling::never(&data_dir, "kal.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(appender);

    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(non_blocking)
        .with_ansi(false);
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(LevelFilter::TRACE)
        .finish()
        .with(layer);
    tracing::subscriber::set_global_default(subscriber)?;

    run(data_dir).inspect_err(|e| tracing::error!("{e}"))
}
