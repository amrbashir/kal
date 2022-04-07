#[path = "../common_types/mod.rs"]
mod common_types;
mod fuzzy_sort;
mod plugin;
mod plugins;

use common_types::{IPCEvent, SearchResultItem};
use fuzzy_sort::fuzzy_sort;
use plugin::Plugin;
use plugins::app_launcher::AppLauncherPlugin;
#[cfg(not(debug_assertions))]
use rust_embed::RustEmbed;
use serde::Serialize;
use std::{
    cell::RefCell,
    sync::atomic::{AtomicU8, Ordering},
};
#[cfg(not(debug_assertions))]
use wry::http::ResponseBuilder;
use wry::{
    application::{
        dpi::{LogicalPosition, LogicalSize},
        event::{Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        platform::windows::WindowBuilderExtWindows,
        window::{WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

#[cfg(not(debug_assertions))]
#[derive(RustEmbed)]
#[folder = "dist"]
struct Asset;

struct AppState {
    search_input_webview: WebView,
    search_results_webview: WebView,
    plugins: Vec<Box<dyn Plugin>>,
    current_results: Vec<SearchResultItem>,
    current_selection: AtomicU8,
}

enum AppEvent {
    /// An Ipc event from the webview
    Ipc(WindowId, String),
}

fn main() {
    let event_loop = EventLoop::<AppEvent>::with_user_event();

    let app_state = RefCell::new(AppState {
        search_input_webview: create_webview("SearchInput", 600, 60, 600, 300, &event_loop),
        search_results_webview: create_webview("SearchResults", 600, 400, 600, 370, &event_loop),
        plugins: vec![AppLauncherPlugin::new()],
        current_results: Vec::new(),
        current_selection: AtomicU8::new(0),
    });

    // refresh plugins
    for plugin in &mut app_state.borrow_mut().plugins {
        plugin.refresh();
    }

    event_loop.run(move |event, _, control_flow| match event {
        Event::WindowEvent { event, .. } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            _ => {}
        },

        Event::UserEvent(event) => match event {
            AppEvent::Ipc(_window_id, request) => {
                let mut s = request.split("::");
                let event: IPCEvent = s.next().unwrap_or_default().into();
                let payload_str = s.next().unwrap_or_default();
                let payload: Vec<&str> =
                    serde_json::from_str::<Vec<&str>>(payload_str).unwrap_or_default();

                match event {
                    IPCEvent::Search => {
                        let mut app_state = app_state.borrow_mut();

                        app_state.current_selection.store(0, Ordering::Relaxed);

                        let mut results = Vec::new();
                        for plugin in &app_state.plugins {
                            results.extend_from_slice(plugin.results(payload[0]));
                        }

                        let sorted_results = fuzzy_sort(payload[0], results);

                        emit_event(
                            &app_state.search_results_webview,
                            IPCEvent::Results.into(),
                            &sorted_results,
                        );

                        app_state.current_results = sorted_results;
                    }

                    IPCEvent::Execute => {
                        let app_state = app_state.borrow();
                        let item = &app_state.current_results
                            [app_state.current_selection.load(Ordering::Relaxed) as usize];

                        app_state
                            .plugins
                            .iter()
                            .filter(|p| p.name() == item.plugin_name)
                            .collect::<Vec<&Box<dyn Plugin>>>()
                            .first()
                            .expect(
                                format!("Failed to find the {} plugin!", item.plugin_name).as_str(),
                            )
                            .execute(item);
                    }

                    IPCEvent::ClearResults => {
                        app_state
                            .borrow_mut()
                            .current_selection
                            .store(0, Ordering::Relaxed);
                        emit_event(
                            &app_state.borrow().search_results_webview,
                            IPCEvent::ClearResults.into(),
                            &"",
                        );
                    }

                    IPCEvent::SelectNextResult => {
                        let app_state = app_state.borrow_mut();
                        let results_len = app_state.current_results.len();
                        let current_selection = app_state.current_selection.load(Ordering::Relaxed);
                        let next_selection;
                        if current_selection == results_len as u8 - 1 {
                            next_selection = 0;
                        } else {
                            next_selection = current_selection + 1;
                        }

                        emit_event(
                            &app_state.search_results_webview,
                            IPCEvent::SelectNextResult.into(),
                            &next_selection,
                        );

                        app_state
                            .current_selection
                            .store(next_selection, Ordering::Relaxed);
                    }

                    IPCEvent::SelectPreviousResult => {
                        let app_state = app_state.borrow_mut();
                        let results_len = app_state.current_results.len() as u8;
                        let current_selection = app_state.current_selection.load(Ordering::Relaxed);
                        let next_selection;
                        if current_selection == 0 {
                            next_selection = results_len - 1;
                        } else {
                            next_selection = 0;
                        }

                        emit_event(
                            &app_state.search_results_webview,
                            IPCEvent::SelectNextResult.into(),
                            &next_selection,
                        );

                        app_state
                            .current_selection
                            .store(next_selection, Ordering::Relaxed);
                    }
                    _ => {}
                }
            }
        },
        _ => {}
    });
}

/// Emits an event to a window
///
/// This invokes the js handlers registred through `window.KAL.ipc.on()`
pub fn emit_event(webview: &WebView, event: &str, payload: &impl Serialize) {
    if webview
        .evaluate_script(
            format!(
                r#"
                (function(){{
                  window.KAL.ipc.__event_handlers['{}'].forEach(handler => {{
                    handler({});
                  }});
                }})()
              "#,
                event,
                serialize_to_javascript::Serialized::new(
                    &serde_json::value::to_raw_value(payload).unwrap_or_default(),
                    &serialize_to_javascript::Options::default()
                ),
            )
            .as_str(),
        )
        .is_err()
    {
        println!("[ERROR][IPC]: failed to emit `{}` event", event);
    };
}

fn create_webview(
    url: &str,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    event_loop: &EventLoop<AppEvent>,
) -> WebView {
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
        .expect(format!("Failed to build {} window!", url).as_str());
    let proxy = event_loop.create_proxy();
    #[allow(unused_mut)]
    let mut webview_builder = WebViewBuilder::new(window)
        .unwrap()
        .with_transparent(true)
        .with_initialization_script(
            r#"
                  Object.defineProperty(window, "KAL", {
                    value: {
                      ipc: {
                        send: (eventName, ...payload) => {
                          window.ipc.postMessage(`${eventName}::${JSON.stringify(payload)}`);
                        },
                        __event_handlers: {},
                        on: function (eventName, event_handler) {
                          if (typeof this.__event_handlers[eventName] == "undefined")
                            this.__event_handlers[eventName] = [];
                          this.__event_handlers[eventName].push(event_handler);
                        },
                      },
                    },
                  });
                "#,
        )
        .with_url(&url)
        .unwrap()
        .with_ipc_handler(move |w, r| {
            let _ = proxy.send_event(AppEvent::Ipc(w.id(), r));
        });
    #[cfg(debug_assertions)]
    {
        webview_builder = webview_builder.with_devtools(true)
    }
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
    webview_builder
        .build()
        .expect(format!("Failed to build {} webview!", url).as_str())
}
