use anyhow::Context;
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

pub fn run() -> anyhow::Result<()> {
    // Use two threads for async
    std::env::set_var("SMOL_THREADS", "2");

    let data_dir = dirs::data_local_dir()
        .context("Failed to get $data_local_dir path")?
        .join("kal");

    let event_loop = EventLoop::new()?;
    event_loop.set_control_flow(ControlFlow::Wait);

    let proxy = event_loop.create_proxy();

    let mut app = crate::app::App::new(data_dir, proxy)?;

    event_loop.run_app(&mut app).map_err(Into::into)
}
