#[path = "../common_types/mod.rs"]
mod common_types;
mod config;
mod fuzzy_sort;
mod plugin;
mod plugins;

use common_types::{IPCEvent, SearchResultItem};
use config::{Config, CONFIG_FILE_NAME};
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
    modifier_pressed: bool,
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
    #[cfg(target_os = "windows")]
    Focus(bool),
}

fn main() {
    let mut config_path = dirs_next::data_local_dir().unwrap();
    config_path.push("kal");
    config_path.push(CONFIG_FILE_NAME);
    let config = Config::load_from_path(config_path);

    let event_loop = EventLoop::<AppEvent>::with_user_event();

    let monitor = event_loop
        .primary_monitor()
        .expect("Failed to get primary monitor.");
    let (m_size, m_pos) = (monitor.size(), monitor.position());

    let main_window = create_webview_window(
        "/main-window",
        config.window_width,
        config.input_height,
        m_pos.x + (m_size.width as i32 / 2 - config.window_width as i32 / 2),
        m_pos.y + (m_size.height as i32 / 4),
        false,
        false,
        false,
        true,
        true,
        &event_loop,
    );
    // disable minimize animation on Windows, so it can feel snappy when hiding or showing it.
    #[cfg(target_os = "windows")]
    {
        use windows_sys::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
        };
        use wry::application::platform::windows::WindowExtWindows;
        unsafe {
            DwmSetWindowAttribute(
                main_window.window().hwnd() as _,
                DWMWA_TRANSITIONS_FORCEDISABLED,
                &1 as *const _ as _,
                4,
            );
        }
    }

    let app_state = RefCell::new(AppState {
        main_window,
        plugins: vec![AppLauncherPlugin::new()],
        current_results: Vec::new(),
        modifier_pressed: false,
    });

    for plugin in &mut app_state.borrow_mut().plugins {
        plugin.refresh();
    }

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent { event, .. } => match event {
            DeviceEvent::Key(k) => {
                let mut app_state = app_state.borrow_mut();
                if k.physical_key.to_string() == config.hotkey.0 {
                    app_state.modifier_pressed = if k.state == ElementState::Pressed {
                        true
                    } else {
                        false
                    };
                }

                if k.physical_key.to_string() == config.hotkey.1
                    && k.state == ElementState::Pressed
                    && app_state.modifier_pressed
                {
                    let main_window = app_state.main_window.window();
                    if main_window.is_visible() {
                        // minimize before hiding to return focus to previous window
                        #[cfg(target_os = "windows")]
                        main_window.set_minimized(true);
                        main_window.set_visible(false);
                    } else {
                        main_window.set_visible(true);
                        #[cfg(target_os = "windows")]
                        main_window.set_minimized(false);
                        main_window.set_focus();
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
            WindowEvent::Focused(focus) => {
                let app_state = app_state.borrow();
                if window_id == app_state.main_window.window().id() && focus {
                    app_state.main_window.focus();
                }

                #[cfg(any(target_os = "linux", target_os = "macos"))]
                {
                    if window_id == app_state.main_window.window().id() && !focus {
                        app_state.main_window.window().set_visible(false);
                    }
                }
            }
            _ => {}
        },
        Event::UserEvent(event) => match event {
            AppEvent::WebviewEvent { event, window_id } => match event {
                #[cfg(target_os = "windows")]
                WebviewEvent::Focus(focus) => {
                    let app_state = app_state.borrow();
                    let main_window = app_state.main_window.window();
                    if window_id == main_window.id() && !focus {
                        main_window.set_visible(false);
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

                        let requested_height =
                            sorted_results.len() as u32 * config.results_item_height;
                        let new_height = if requested_height >= config.results_height {
                            config.results_height
                        } else {
                            requested_height
                        };
                        app_state
                            .main_window
                            .window()
                            .set_inner_size(LogicalSize::new(config.window_width, new_height));
                        let _ = app_state.main_window.resize();

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
                        let mut app_state = app_state.borrow_mut();
                        app_state
                            .main_window
                            .window()
                            .set_inner_size(LogicalSize::new(
                                config.window_width,
                                config.input_height,
                            ));
                        let _ = app_state.main_window.resize();
                        app_state.current_results = Vec::new();
                    }
                    IPCEvent::HideMainWindow => {
                        let app_state = app_state.borrow();
                        let main_window = app_state.main_window.window();
                        #[cfg(target_os = "windows")]
                        main_window.set_minimized(true);
                        main_window.set_visible(false);
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
fn emit_event(webview: &WebView, event: &str, payload: &impl Serialize) {
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
    x: i32,
    y: i32,
    decorated: bool,
    visible: bool,
    resizable: bool,
    transparent: bool,
    skip_taskbar: bool,
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
        .with_inner_size(LogicalSize::new(width, height))
        .with_position(LogicalPosition::new(x, y))
        .with_decorations(decorated)
        .with_resizable(resizable)
        .with_visible(visible);
    #[cfg(any(target_os = "linux", target_os = "windows",))]
    {
        window_builder = window_builder.with_skip_taskbar(skip_taskbar);
    }
    let window = window_builder
        .with_transparent(transparent)
        .build(event_loop)
        .expect(format!("Failed to build {} window!", url).as_str());

    let proxy = event_loop.create_proxy();
    #[allow(unused_mut)]
    let mut webview_builder = WebViewBuilder::new(window)
        .unwrap()
        .with_transparent(transparent)
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
