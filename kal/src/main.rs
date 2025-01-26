use std::path::PathBuf;

use anyhow::Context;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
#[cfg(not(debug_assertions))]
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
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

pub fn run(data_dir: PathBuf) -> anyhow::Result<()> {
    // Use two threads for async
    std::env::set_var("SMOL_THREADS", "2");

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

    let env_filter = EnvFilter::try_from_env("KAL_LOG").unwrap_or_else(|_| {
        EnvFilter::builder()
            .with_default_directive(LevelFilter::DEBUG.into())
            .from_env_lossy()
    });

    let subscriber = tracing_subscriber::fmt()
        .compact()
        .with_span_events(FmtSpan::ACTIVE)
        .with_max_level(tracing::Level::TRACE)
        .with_target(false)
        .with_env_filter(env_filter)
        .finish();

    #[cfg(not(debug_assertions))]
    let (chrome_layer, _c_guard) = {
        let appender = tracing_appender::rolling::daily(&data_dir.join("logs"), "kal.trace");
        let (layer, _guard) = tracing_chrome::ChromeLayerBuilder::new()
            .writer(appender)
            .build();
        (layer, _guard)
    };

    #[cfg(not(debug_assertions))]
    let (file_log_layer, _f_guard) = {
        let appender = tracing_appender::rolling::daily(&data_dir.join("logs"), "kal.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(appender);
        let layer = tracing_subscriber::fmt::Layer::default()
            // disable ansi coloring in log file
            .with_ansi(false)
            .with_writer(non_blocking);

        (layer, _guard)
    };

    #[cfg(not(debug_assertions))]
    let subscriber = subscriber.with(chrome_layer);
    #[cfg(not(debug_assertions))]
    let subscriber = subscriber.with(file_log_layer);

    tracing::subscriber::set_global_default(subscriber)?;

    tracing::debug!("Logger initialized");

    run(data_dir).inspect_err(|e| tracing::error!("{e}"))
}
