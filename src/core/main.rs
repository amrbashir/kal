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
use std::cell::RefCell;
#[cfg(not(debug_assertions))]
use wry::http::ResponseBuilder;
use wry::{
    application::{
        dpi::{LogicalPosition, LogicalSize},
        event::{DeviceEvent, ElementState, Event, WindowEvent},
        event_loop::{ControlFlow, EventLoop},
        keyboard::KeyCode,
        window::{WindowBuilder, WindowId},
    },
    webview::{WebView, WebViewBuilder},
};

#[cfg(not(debug_assertions))]
#[derive(RustEmbed)]
#[folder = "dist"]
struct Asset;

struct AppState {
    main_window: WebView,
    plugins: Vec<Box<dyn Plugin>>,
    current_results: Vec<SearchResultItem>,
    alt_key_pressed: bool,
}

enum AppEvent {
    /// An Ipc event from the webview
    Ipc(WindowId, String),
    /// Describes an event from a [`WebView`]
    WebviewEvent {
        event: WebviewEvent,
        window_id: WindowId,
    },
}

enum WebviewEvent {
    /// The webview gained or lost focus
    ///
    /// Currently, it is only used on Windows
    Focus(bool),
}

fn main() {
    let event_loop = EventLoop::<AppEvent>::with_user_event();

    let app_state = RefCell::new(AppState {
        main_window: create_webview_window("/main-window", 600, 460, 600, 300, &event_loop),
        plugins: vec![AppLauncherPlugin::new()],
        current_results: Vec::new(),
        alt_key_pressed: false,
    });

    for plugin in &mut app_state.borrow_mut().plugins {
        plugin.refresh();
    }

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::Key(k) => {
                let mut app_state = app_state.borrow_mut();
                if k.physical_key == KeyCode::AltLeft {
                    app_state.alt_key_pressed = if k.state == ElementState::Pressed {
                        true
                    } else {
                        false
                    };
                }

                if k.physical_key == KeyCode::Space
                    && k.state == ElementState::Pressed
                    && app_state.alt_key_pressed
                {
                    let window = app_state.main_window.window();
                    if window.is_visible() {
                        window.set_minimized(true);
                        window.set_visible(false);
                    } else {
                        window.set_visible(true);
                        window.set_minimized(false);
                        window.set_focus();
                        emit_event(&app_state.main_window, IPCEvent::FocusInput.into(), &"");
                    }
                }
            }
            _ => {}
        },
        #[allow(unused)]
        Event::WindowEvent {
            event, window_id, ..
        } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Focused(f) => {
                let app_state = app_state.borrow();
                if window_id == app_state.main_window.window().id() && f {
                    app_state.main_window.focus();
                }

                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    // when main window loses focus
                    if window_id == app_state.main_window.window().id() && !f {
                        app_state.main_window.window().set_visible(false);
                    }
                }
            }
            _ => {}
        },
        Event::UserEvent(event) => match event {
            AppEvent::WebviewEvent { event, window_id } => match event {
                WebviewEvent::Focus(f) => {
                    let app_state = app_state.borrow();
                    // when main window loses focus
                    if window_id == app_state.main_window.window().id() && !f {
                        app_state.main_window.window().set_visible(false);
                    }
                }
            },
            AppEvent::Ipc(_window_id, request) => {
                let mut s = request.split("::");
                let event: IPCEvent = s.next().unwrap_or_default().into();
                let payload_str = s.next().unwrap_or_default();

                match event {
                    IPCEvent::Search => {
                        let query = serde_json::from_str::<Vec<&str>>(payload_str).unwrap()[0];

                        let mut app_state = app_state.borrow_mut();

                        let mut results = Vec::new();
                        for plugin in &app_state.plugins {
                            results.extend_from_slice(plugin.results(query));
                        }

                        let sorted_results = fuzzy_sort(query, results);

                        emit_event(
                            &app_state.main_window,
                            IPCEvent::Results.into(),
                            &sorted_results,
                        );

                        app_state.current_results = sorted_results;
                    }
                    IPCEvent::Execute => {
                        let index = serde_json::from_str::<Vec<usize>>(payload_str).unwrap()[0];

                        let app_state = app_state.borrow();
                        let item = &app_state.current_results[index];

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
                        app_state.borrow_mut().current_results = Vec::new();
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

fn create_webview_window(
    url: &str,
    width: u32,
    height: u32,
    x: u32,
    y: u32,
    event_loop: &EventLoop<AppEvent>,
) -> WebView {
    #[cfg(target_os = "linux")]
    use wry::application::platform::unix::WindowBuilderExtWindows;
    #[cfg(target_os = "windows")]
    use wry::application::platform::windows::WindowBuilderExtWindows;

    #[cfg(debug_assertions)]
    let url = format!("http://localhost:9010{}", url);
    #[cfg(not(debug_assertions))]
    let url = format!("kal://localhost/{}", url);

    let mut window_builder = WindowBuilder::new()
        .with_inner_size(LogicalSize::<u32>::new(width, height))
        .with_position(LogicalPosition::<u32>::new(x, y))
        .with_decorations(false)
        .with_resizable(false)
        .with_visible(false);
    #[cfg(any(target_os = "linux", target_os = "windows",))]
    {
        window_builder = window_builder.with_skip_taskbar(true);
    }
    let window = window_builder
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
    let webview = webview_builder
        .build()
        .expect(format!("Failed to build {} webview!", url).as_str());

    #[cfg(target_os = "windows")]
    {
        use wry::webview::WebviewExtWindows;
        let mut token = unsafe { std::mem::zeroed() };
        let controller = webview.controller();
        let window_id = webview.window().id();
        unsafe {
            let proxy = event_loop.create_proxy();
            let _ = controller.GotFocus(
                webview2_com::FocusChangedEventHandler::create(Box::new(move |_, _| {
                    let _ = proxy.send_event(AppEvent::WebviewEvent {
                        event: WebviewEvent::Focus(true),
                        window_id,
                    });
                    Ok(())
                })),
                &mut token,
            );
            let proxy = event_loop.create_proxy();
            let _ = controller.LostFocus(
                webview2_com::FocusChangedEventHandler::create(Box::new(move |_, _| {
                    let _ = proxy.send_event(AppEvent::WebviewEvent {
                        event: WebviewEvent::Focus(false),
                        window_id,
                    });
                    Ok(())
                })),
                &mut token,
            );
        }
    }

    webview
}
