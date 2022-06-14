#[path = "../common_types/mod.rs"]
mod common_types;
mod config;
mod event;
mod fuzzy_sort;
mod plugin;
mod plugins;
mod webview_window;

use common_types::{IPCEvent, SearchResultItem};
use config::Config;
use event::{emit_event, AppEvent, WebviewEvent, INIT_SCRIPT};
use fuzzy_sort::fuzzy_sort;
use plugin::Plugin;
use plugins::app_launcher::AppLauncherPlugin;
#[cfg(not(debug_assertions))]
use rust_embed::RustEmbed;
use webview_window::WebviewWindow;
use wry::application::window::WindowAttributes;
use wry::application::{
    dpi::{LogicalPosition, LogicalSize},
    event::{DeviceEvent, ElementState, Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
};
use wry::webview::WebViewAttributes;

#[cfg(not(debug_assertions))]
#[derive(RustEmbed)]
#[folder = "dist"]
pub(crate) struct EmbededAsset;

struct AppState {
    main_window: WebviewWindow,
    #[cfg(target_os = "windows")]
    previously_foreground_hwnd: windows_sys::Win32::Foundation::HWND,
    plugins: Vec<Box<dyn Plugin>>,
    current_results: Vec<SearchResultItem>,
    modifier_pressed: bool,
}

fn main() -> anyhow::Result<()> {
    let config = Config::load()?;
    let plugins: Vec<Box<dyn Plugin>> = vec![AppLauncherPlugin::new(&config)];
    let event_loop = EventLoop::<AppEvent>::with_user_event();
    let main_window = create_main_window(&config, &event_loop)?;
    let app_state = std::cell::RefCell::new(AppState {
        main_window,
        #[cfg(target_os = "windows")]
        previously_foreground_hwnd: 0,
        plugins,
        current_results: Default::default(),
        modifier_pressed: false,
    });

    for plugin in &mut app_state.borrow_mut().plugins {
        plugin.refresh();
    }

    event_loop.run(move |event, _, control_flow| match event {
        // handle hotkey
        Event::DeviceEvent {
            event: DeviceEvent::Key(k),
            ..
        } => {
            let mut app_state = app_state.borrow_mut();
            let (modifier, key) = &config.general.hotkey;

            if &k.physical_key.to_string() == modifier {
                app_state.modifier_pressed = k.state == ElementState::Pressed;
            }

            if &k.physical_key.to_string() == key
                && k.state == ElementState::Pressed
                && app_state.modifier_pressed
            {
                #[cfg(target_os = "windows")]
                {
                    app_state.previously_foreground_hwnd = unsafe {
                        windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow()
                    };
                }
                toggle_main_window(&mut app_state);
            }
        }

        Event::UserEvent(event) => match event {
            AppEvent::Ipc(_window_id, request) => {
                let r = request.split("::").take(2).collect::<Vec<_>>();
                let (event, payload): (IPCEvent, &str) = (r[0].into(), r[1]);

                match event {
                    IPCEvent::Search => {
                        let mut app_state = app_state.borrow_mut();

                        let query =
                            serde_json::from_str::<Vec<&str>>(payload).unwrap_or_default()[0];
                        let mut results = Vec::new();
                        for plugin in app_state.plugins.iter().filter(|p| p.enabled()) {
                            results.extend_from_slice(plugin.results(query));
                        }

                        fuzzy_sort!(results, primary_text, query);

                        let max_results = results
                            .iter()
                            .take(config.general.max_search_results as usize)
                            .collect::<Vec<_>>();

                        emit_event(
                            &app_state.main_window,
                            IPCEvent::Results.into(),
                            &max_results,
                        );

                        resize_main_window_for_results(
                            &app_state.main_window,
                            &config,
                            max_results.len(),
                        );

                        app_state.current_results = results;
                    }
                    IPCEvent::Execute => {
                        let app_state = app_state.borrow();

                        let (index, elevated) =
                            serde_json::from_str::<(usize, bool)>(payload).unwrap();
                        let item = &app_state.current_results[index];
                        app_state
                            .plugins
                            .iter()
                            .filter(|p| p.name() == item.plugin_name)
                            .collect::<Vec<&Box<dyn Plugin>>>()
                            .first()
                            .unwrap_or_else(|| panic!("Failed to find  {}!", item.plugin_name))
                            .execute(item, elevated);

                        hide_main_window(&app_state, false);
                    }
                    IPCEvent::OpenLocation => {
                        let app_state = app_state.borrow();

                        let index = serde_json::from_str::<(usize,)>(payload).unwrap().0;
                        let item = &app_state.current_results[index];
                        app_state
                            .plugins
                            .iter()
                            .filter(|p| p.name() == item.plugin_name)
                            .collect::<Vec<&Box<dyn Plugin>>>()
                            .first()
                            .unwrap_or_else(|| panic!("Failed to find  {}!", item.plugin_name))
                            .open_location(item);

                        hide_main_window(&app_state, false);
                    }
                    IPCEvent::ClearResults => {
                        let mut app_state = app_state.borrow_mut();

                        resize_main_window_for_results(&app_state.main_window, &config, 0);

                        app_state.current_results = Vec::new();
                    }
                    IPCEvent::HideMainWindow => {
                        hide_main_window(&app_state.borrow(), true);
                    }
                    _ => {}
                }
            }

            AppEvent::WebviewEvent { event, window_id } => match event {
                #[cfg(target_os = "windows")]
                WebviewEvent::Focus(focus) => {
                    let app_state = app_state.borrow();
                    let main_window = app_state.main_window.window();
                    // hide main window when the webview loses focus
                    if window_id == main_window.id() && !focus {
                        main_window.set_visible(false);
                    }
                }
            },
        },

        #[allow(unused)]
        Event::WindowEvent {
            event, window_id, ..
        } => match event {
            WindowEvent::CloseRequested => *control_flow = ControlFlow::Exit,
            WindowEvent::Focused(focus) => {
                let app_state = app_state.borrow();
                let main_window = app_state.main_window.window();

                // hide main window when it loses focus
                #[cfg(any(target_os = "linux", target_os = "macos"))]
                if window_id == main_window.id() && !focus {
                    main_window.set_visible(false);
                }

                // because On Windows, the window and the webview have differnet focus
                // we have to focus the webview, when the window gains focus so input
                // elements and similar can take focus correcty.
                if window_id == main_window.id() && focus {
                    app_state.main_window.focus();
                }
            }
            _ => {}
        },

        _ => {}
    });
}

fn create_main_window(
    config: &Config,
    event_loop: &EventLoop<AppEvent>,
) -> anyhow::Result<WebviewWindow> {
    #[cfg(target_os = "linux")]
    use wry::application::platform::linux::WindowExtWindows;
    #[cfg(target_os = "windows")]
    use wry::application::platform::windows::WindowExtWindows;

    let (m_size, m_pos) = {
        let monitor = event_loop
            .primary_monitor()
            .expect("Failed to get primary monitor");
        (monitor.size(), monitor.position())
    };

    #[cfg(debug_assertions)]
    let url = "http://localhost:9010/main-window";
    #[cfg(not(debug_assertions))]
    let url = "kal://localhost/main-window";

    let main_window = WebviewWindow::new(
        WindowAttributes {
            inner_size: Some(
                LogicalSize::new(
                    config.appearance.window_width,
                    config.appearance.input_height,
                )
                .into(),
            ),
            position: Some(
                LogicalPosition::new(
                    m_pos.x + (m_size.width as i32 / 2 - config.appearance.window_width as i32 / 2),
                    m_pos.y + (m_size.height as i32 / 4),
                )
                .into(),
            ),
            decorations: false,
            resizable: false,
            visible: false,
            transparent: config.appearance.transparent,

            ..Default::default()
        },
        WebViewAttributes {
            url: Some(url.parse().unwrap()),
            transparent: config.appearance.transparent,
            initialization_scripts: vec![INIT_SCRIPT.to_string()],
            devtools: cfg!(debug_assertions),
            ipc_handler: Some({
                let proxy = event_loop.create_proxy();
                Box::new(move |w, r| {
                    let _ = proxy.send_event(AppEvent::Ipc(w.id(), r));
                })
            }),
            ..Default::default()
        },
        &event_loop,
    )?;

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    main_window.window().set_skip_taskbar(true);

    Ok(main_window)
}

fn toggle_main_window(app_state: &mut AppState) {
    let window = app_state.main_window.window();
    if window.is_visible() {
        hide_main_window(app_state, true);
    } else {
        show_main_window(app_state);
    }
}

fn show_main_window(app_state: &mut AppState) {
    #[cfg(target_os = "windows")]
    {
        app_state.previously_foreground_hwnd =
            unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    }
    let main_window = &app_state.main_window;
    let window = main_window.window();
    window.set_visible(true);
    window.set_focus();
    emit_event(&main_window, IPCEvent::FocusInput.into(), &"");
}

fn hide_main_window(app_state: &AppState, #[allow(unused)] restore_focus: bool) {
    app_state.main_window.window().set_visible(false);
    #[cfg(target_os = "windows")]
    {
        if restore_focus {
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetForegroundWindow(
                    app_state.previously_foreground_hwnd,
                )
            };
        }
    }
}

fn resize_main_window_for_results(main_window: &WebviewWindow, config: &Config, count: usize) {
    main_window.window().set_inner_size(LogicalSize::new(
        config.appearance.window_width,
        std::cmp::min(
            count as u32 * config.appearance.results_item_height,
            config.appearance.results_height,
        ) + config.appearance.input_height,
    ));
    let _ = main_window.resize();
}
