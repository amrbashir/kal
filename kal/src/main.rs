#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::fmt::Display;
use std::path::PathBuf;

use anyhow::Context;
use kal_utils::open_url;
use tracing::level_filters::LevelFilter;
use tracing_subscriber::fmt::format::FmtSpan;
#[cfg(not(debug_assertions))]
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::EnvFilter;
use winit::event_loop::{ControlFlow, EventLoop};

mod app;
#[cfg(not(debug_assertions))]
mod embedded_assets;
mod icon;
mod ipc;
mod main_window;
mod plugin_manager;
mod webview_window;

fn error_dialog<T: Display>(error: T) {
    rfd::MessageDialog::new()
        .set_title("komorebi-switcher")
        .set_description(error.to_string())
        .set_level(rfd::MessageLevel::Error)
        .set_buttons(rfd::MessageButtons::Ok)
        .show();
}

const WEBVIEW2_DOWNLOAD_LINK: &str = "https://go.microsoft.com/fwlink/p/?LinkId=2124703";

pub fn run(data_dir: PathBuf) -> anyhow::Result<()> {
    if wry::webview_version().is_err() {
        let res = rfd::MessageDialog::new()
            .set_title("Missing WebView2 Runtime")
            .set_description(
                format!("This application requires WebView2 Runtime.\nDownload from {WEBVIEW2_DOWNLOAD_LINK}")
            )
            .set_level(rfd::MessageLevel::Error)
            .set_buttons(rfd::MessageButtons::OkCancel)
            .show();

        if res == rfd::MessageDialogResult::Ok {
            open_url(&WEBVIEW2_DOWNLOAD_LINK.parse()?)?;
        }

        std::process::exit(1);
    }

    // Use two threads for async
    std::env::set_var("SMOL_THREADS", "2");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();

    let mut app = crate::app::App::new(data_dir, proxy)?;

    event_loop.run_app(&mut app).map_err(Into::into)
}

fn main() -> anyhow::Result<()> {
    let data_dir = dirs::data_dir()
        .context("Failed to get $data_dir path")?
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

    std::panic::set_hook(Box::new(|info| {
        error_dialog(info);
        tracing::error!("{info}");
    }));

    if let Err(e) = run(data_dir) {
        error_dialog(&e);
        tracing::error!("{e}");
        std::process::exit(1);
    }

    Ok(())
}
