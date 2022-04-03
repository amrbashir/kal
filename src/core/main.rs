mod ipc;

use ipc::{emit_event, handle_ipc, KAL_IPC_SCRIPT};
#[cfg(not(debug_assertions))]
use rust_embed::RustEmbed;
use std::{cell::RefCell, collections::HashMap};
use wry::application::event::{Event, WindowEvent};
use wry::application::event_loop::ControlFlow;
#[cfg(not(debug_assertions))]
use wry::http::ResponseBuilder;
use wry::{
    application::{
        dpi::{LogicalPosition, LogicalSize},
        event_loop::EventLoop,
        platform::windows::WindowBuilderExtWindows,
        window::{Window, WindowBuilder},
    },
    webview::{WebView, WebViewBuilder},
};

thread_local! {
  static WEBVIEWS: RefCell< HashMap<u8, WebView>> = RefCell::new(HashMap::new());
}

const SEARCH_INPUT_WINDOW_ID: u8 = 1;
const SEARCH_RESULTS_WINDOW_ID: u8 = 2;

#[cfg(not(debug_assertions))]
#[derive(RustEmbed)]
#[folder = "dist"]
struct Asset;

/// Handles events sent by a window through `window.KAL.ipc.send()`
fn on_ipc_event(_window: &Window, event_name: &str, payload: Vec<&str>) {
    if event_name == "search" {
        emit_event(
            SEARCH_RESULTS_WINDOW_ID,
            "results",
            &vec![
                "next item is the query",
                payload[0],
                "previous item is the query",
            ],
        );
    }
}

fn create_webview<T>(
    url: &str,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    id: u8,
    event_loop: &EventLoop<T>,
) {
    #[cfg(debug_assertions)]
    let url = format!("http://localhost:9010/{}", url);
    #[cfg(not(debug_assertions))]
    let url = format!("kal://localhost/{}/", url);

    let window = WindowBuilder::new()
        .with_inner_size(LogicalSize::<u32>::new(width, height))
        .with_position(LogicalPosition::<u32>::new(x, y))
        .with_decorations(false)
        .with_resizable(false)
        .with_skip_taskbar(true)
        .with_transparent(true)
        .build(event_loop)
        .unwrap();
    #[allow(unused_mut)]
    let mut webview_builder = WebViewBuilder::new(window)
        .unwrap()
        .with_transparent(true)
        .with_initialization_script(KAL_IPC_SCRIPT)
        .with_url(&url)
        .unwrap()
        .with_ipc_handler(|w, r| handle_ipc(w, r, on_ipc_event));

    #[cfg(not(debug_assertions))]
    {
        webview_builder = webview_builder.with_custom_protocol("kal".into(), move |request| {
            use std::path::PathBuf;

            let path = request.uri().replace("kal://localhost/", "");
            let data = Asset::get(&path)
                .unwrap_or_else(|| Asset::get("index.html").unwrap())
                .data;
            let mimetype = match PathBuf::from(path)
                .extension()
                .unwrap_or_default()
                .to_str()
                .unwrap_or_default()
            {
                "html" | "htm" => "text/html",
                "js" | "mjs" => "text/javascript",
                "css" => "text/css",
                "png" => "image/png",
                "jpg" | "jpeg" => "image/jpeg",
                "svg" => "image/svg+xml",
                _ => "text/html",
            };

            ResponseBuilder::new()
                .mimetype(mimetype)
                .body(data.to_vec())
        })
    }
    let webview = webview_builder.build().unwrap();

    WEBVIEWS.with(|webviews| {
        let mut webviews = webviews.borrow_mut();
        webviews.insert(id, webview);
    });
}

fn main() {
    let event_loop = EventLoop::new();

    create_webview(
        "SearchInput",
        600,
        60,
        600,
        300,
        SEARCH_INPUT_WINDOW_ID,
        &event_loop,
    );
    create_webview(
        "SearchResults",
        600,
        400,
        600,
        370,
        SEARCH_RESULTS_WINDOW_ID,
        &event_loop,
    );

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            _ => {}
        },
        _ => {}
    });
}
