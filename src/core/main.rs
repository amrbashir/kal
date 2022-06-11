#[path = "../common_types/mod.rs"]
mod common_types;
mod config;
mod fuzzy_sort;
mod plugin;
mod plugins;

use common_types::{IPCEvent, SearchResultItem};
use config::Config;
use fuzzy_sort::fuzzy_sort;
use plugin::Plugin;
use plugins::app_launcher::AppLauncherPlugin;
#[cfg(not(debug_assertions))]
use rust_embed::RustEmbed;
use serde::Serialize;
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
struct EmbededAsset;

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
    let config = Config::load();

    let event_loop = EventLoop::<AppEvent>::with_user_event();

    let (m_size, m_pos) = {
        let monitor = event_loop
            .primary_monitor()
            .expect("Failed to get primary monitor");
        (monitor.size(), monitor.position())
    };
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
    // disable minimize animation on Windows so that hiding or showing the main window can feel snappy.
    //
    // why minimize the window in the first place?
    // on Windows, minimizing the main window before hiding it,
    // serves as a wrokaround to retsore focus correctly to the previous window.
    //
    // TODO: save last focused window before showing the window, and return focus on exiting
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

    let app_state = std::cell::RefCell::new(AppState {
        main_window,
        plugins: vec![AppLauncherPlugin::new()],
        current_results: Vec::new(),
        modifier_pressed: false,
    });

    for plugin in &mut app_state.borrow_mut().plugins {
        plugin.refresh();
    }

    event_loop.run(move |event, _, control_flow| match event {
        Event::DeviceEvent {
            event: DeviceEvent::Key(k),
            ..
        } => {
            let mut app_state = app_state.borrow_mut();
            if k.physical_key.to_string() == config.hotkey.0 {
                app_state.modifier_pressed = k.state == ElementState::Pressed;
            }

            if k.physical_key.to_string() == config.hotkey.1
                && k.state == ElementState::Pressed
                && app_state.modifier_pressed
            {
                let main_window = app_state.main_window.window();
                if main_window.is_visible() {
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
                let payload = s.next().unwrap_or_default();

                match event {
                    IPCEvent::Search => {
                        let query = serde_json::from_str::<Vec<&str>>(payload).unwrap()[0];

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
                        let args = serde_json::from_str::<(usize, bool)>(payload).unwrap();
                        let index = args.0;
                        let elevated = args.1;

                        let app_state = app_state.borrow();
                        let item = &app_state.current_results[index];

                        app_state
                            .plugins
                            .iter()
                            .filter(|p| p.name() == item.plugin_name)
                            .collect::<Vec<&Box<dyn Plugin>>>()
                            .first()
                            .unwrap_or_else(|| panic!("Failed to find  {}!", item.plugin_name))
                            .execute(item, elevated);
                        let main_window = app_state.main_window.window();
                        #[cfg(target_os = "windows")]
                        main_window.set_minimized(true);
                        main_window.set_visible(false);
                    }
                    IPCEvent::OpenLocation => {
                        let index = serde_json::from_str::<Vec<usize>>(payload).unwrap()[0];

                        let app_state = app_state.borrow();
                        let item = &app_state.current_results[index];

                        app_state
                            .plugins
                            .iter()
                            .filter(|p| p.name() == item.plugin_name)
                            .collect::<Vec<&Box<dyn Plugin>>>()
                            .first()
                            .unwrap_or_else(|| panic!("Failed to find  {}!", item.plugin_name))
                            .open_location(item);
                        let main_window = app_state.main_window.window();
                        #[cfg(target_os = "windows")]
                        main_window.set_minimized(true);
                        main_window.set_visible(false);
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
        .unwrap_or_else(|_| panic!("Failed to build {} window!", url));

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
        })
        .with_custom_protocol("kalasset".into(), move |request| {
            let path = request.uri().replace("kalasset://localhost/", "");
            let path = percent_encoding::percent_decode_str(&path).decode_utf8_lossy();
            let path =
                dunce::canonicalize(std::path::PathBuf::from(path.to_string())).unwrap_or_default();

            let mut assets_dir = dirs_next::home_dir().expect("Failed to get $HOME dir path");
            assets_dir.push(".kal");

            if path.starts_with(assets_dir) {
                let mimetype = match path
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
                {
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "svg" => "image/svg+xml",
                    _ => "text/html",
                };

                ResponseBuilder::new()
                    .mimetype(mimetype)
                    .body(std::fs::read(path).unwrap_or_default())
            } else {
                ResponseBuilder::new().body([].into())
            }
        });
    #[cfg(debug_assertions)]
    {
        webview_builder = webview_builder.with_devtools(true)
    }
    #[cfg(not(debug_assertions))]
    {
        webview_builder = webview_builder.with_custom_protocol("kal".into(), move |request| {
            let path = request.uri().replace("kal://localhost/", "");
            let data = EmbededAsset::get(&path)
                .unwrap_or_else(|| EmbededAsset::get("index.html").unwrap())
                .data;
            let mimetype = match std::path::PathBuf::from(path)
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
        .unwrap_or_else(|_| panic!("Failed to build {} webview", url));

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
