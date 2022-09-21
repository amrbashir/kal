#[path = "../common/mod.rs"]
mod common;
mod config;
mod event;
mod fuzzy_sort;
mod plugin;
mod plugins;
mod webview_window;

use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};

use crate::{
    common::{IPCEvent, SearchResultItem},
    config::Config,
    event::{emit_event, AppEvent, ThreadEvent, WebviewEvent, KAL_IPC_INIT_SCRIPT},
    fuzzy_sort::fuzzy_sort,
    plugin::Plugin,
    plugins::app_launcher::AppLauncherPlugin,
    webview_window::WebviewWindow,
};

use anyhow::Context;
use once_cell::sync::Lazy;
use plugins::directory_indexer::DirectoryIndexerPlugin;
#[cfg(not(debug_assertions))]
use rust_embed::RustEmbed;
use wry::{
    application::{
        dpi::{LogicalPosition, LogicalSize},
        event::{DeviceEvent, ElementState, Event, WindowEvent},
        event_loop::{ControlFlow, DeviceEventFilter, EventLoop, EventLoopProxy},
        window::WindowAttributes,
    },
    webview::WebViewAttributes,
};

static KAL_DATA_DIR: Lazy<PathBuf> = Lazy::new(|| {
    dirs_next::data_local_dir()
        .expect("Failed to get $data_local_dir path")
        .join("kal")
});
static CONFIG_FILE: Lazy<PathBuf> = Lazy::new(|| {
    dirs_next::home_dir()
        .expect("Failed to get $home_dir path")
        .join(".config")
        .join("kal.conf.json")
});

#[cfg(not(debug_assertions))]
#[derive(RustEmbed)]
#[folder = "dist"]
pub(crate) struct EmbededAssets;

#[derive(Debug)]
struct AppState<T: 'static> {
    config: Config,
    main_window: WebviewWindow,
    #[cfg(windows)]
    previously_foreground_hwnd: windows_sys::Win32::Foundation::HWND,
    plugins: Arc<Mutex<Vec<Box<dyn Plugin + Send + 'static>>>>,
    current_results: Vec<SearchResultItem>,
    modifier_pressed: bool,
    proxy: EventLoopProxy<T>,
}

#[tracing::instrument]
fn create_main_window(
    config: &Config,
    event_loop: &EventLoop<AppEvent>,
) -> anyhow::Result<WebviewWindow> {
    #[cfg(target_os = "linux")]
    use wry::application::platform::linux::WindowExtWindows;
    #[cfg(windows)]
    use wry::application::platform::windows::WindowExtWindows;

    let (m_size, m_pos) = {
        let monitor = event_loop
            .primary_monitor()
            .with_context(|| "Failed to get primary monitor")?;
        (monitor.size(), monitor.position())
    };

    #[cfg(debug_assertions)]
    let url = "http://localhost:9010/main";
    #[cfg(not(debug_assertions))]
    let url = "kal://localhost/main";

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
            initialization_scripts: vec![KAL_IPC_INIT_SCRIPT.to_string()],
            devtools: cfg!(debug_assertions),
            ipc_handler: Some({
                let proxy = event_loop.create_proxy();
                Box::new(move |w, r| {
                    if let Err(e) = proxy.send_event(AppEvent::Ipc(w.id(), r)) {
                        tracing::error!("{e}");
                    }
                })
            }),
            ..Default::default()
        },
        event_loop,
    )?;

    #[cfg(any(target_os = "windows", target_os = "linux"))]
    main_window.window().set_skip_taskbar(true);
    Ok(main_window)
}

// Saves the current foreground window then shows the main window
fn show_main_window(app_state: &mut AppState<AppEvent>) {
    #[cfg(windows)]
    {
        app_state.previously_foreground_hwnd =
            unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
    }
    let main_window = &app_state.main_window;
    let window = main_window.window();
    window.set_visible(true);
    window.set_focus();
    emit_event(main_window, IPCEvent::FocusInput, &"");
}

/// Hides the main window and restores focus to the previous foreground window if needed
fn hide_main_window<T>(app_state: &AppState<T>, #[allow(unused)] restore_focus: bool) {
    app_state.main_window.window().set_visible(false);
    #[cfg(windows)]
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

// Resizes the main window based on the number of current results and user config
fn resize_main_window_for_results(main_window: &WebviewWindow, config: &Config, count: usize) {
    main_window.window().set_inner_size(LogicalSize::new(
        config.appearance.window_width,
        std::cmp::min(
            count as u32 * config.appearance.results_item_height,
            config.appearance.results_height,
        ) + config.appearance.input_height,
    ));
}

fn process_events(
    event: &Event<AppEvent>,
    app_state: &RefCell<AppState<AppEvent>>,
) -> anyhow::Result<()> {
    match event {
        // handle hotkey
        Event::DeviceEvent {
            event: DeviceEvent::Key(k),
            ..
        } => {
            let mut app_state = app_state.borrow_mut();
            let (modifier, key) = app_state.config.general.hotkey.clone();

            if k.physical_key.to_string() == modifier {
                app_state.modifier_pressed = k.state == ElementState::Pressed;
            }

            if k.physical_key.to_string() == key
                && k.state == ElementState::Pressed
                && app_state.modifier_pressed
            {
                let window = app_state.main_window.window();
                if window.is_visible() {
                    hide_main_window(&app_state, true);
                } else {
                    show_main_window(&mut app_state);
                }
            }
        }

        Event::UserEvent(event) => match event {
            AppEvent::Ipc(window_id, request)
                if *window_id == app_state.borrow().main_window.window().id() =>
            {
                let r = request.split("::").take(2).collect::<Vec<_>>();
                let (event, payload): (IPCEvent, &str) = (r[0].into(), r[1]);

                match event {
                    IPCEvent::Search => {
                        let mut app_state = app_state.borrow_mut();

                        let query = serde_json::from_str::<Vec<&str>>(payload)?[0];
                        let mut results = Vec::new();
                        for plugin in app_state
                            .plugins
                            .lock()
                            .unwrap()
                            .iter()
                            .filter(|p| p.enabled())
                        {
                            results.extend_from_slice(plugin.results(query));
                        }

                        fuzzy_sort!(results, primary_text, query);

                        let max_results = results
                            .iter()
                            .take(app_state.config.general.max_search_results as usize)
                            .collect::<Vec<_>>();

                        emit_event(&app_state.main_window, IPCEvent::Results, &max_results);

                        resize_main_window_for_results(
                            &app_state.main_window,
                            &app_state.config,
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
                            .lock()
                            .unwrap()
                            .iter()
                            .filter(|p| p.name() == item.plugin_name && p.enabled())
                            .collect::<Vec<&Box<dyn Plugin + Send + 'static>>>()
                            .first()
                            .with_context(|| format!("Failed to find  {}!", item.plugin_name))?
                            .execute(item, elevated);

                        hide_main_window(&app_state, false);
                    }
                    IPCEvent::OpenLocation => {
                        let app_state = app_state.borrow();

                        let index = serde_json::from_str::<(usize,)>(payload).unwrap().0;
                        let item = &app_state.current_results[index];
                        app_state
                            .plugins
                            .lock()
                            .unwrap()
                            .iter()
                            .filter(|p| p.name() == item.plugin_name)
                            .collect::<Vec<&Box<dyn Plugin + Send + 'static>>>()
                            .first()
                            .with_context(|| format!("Failed to find  {}!", item.plugin_name))?
                            .open_location(item);

                        hide_main_window(&app_state, false);
                    }
                    IPCEvent::ClearResults => {
                        let mut app_state = app_state.borrow_mut();
                        app_state.current_results = Vec::new();
                        resize_main_window_for_results(
                            &app_state.main_window,
                            &app_state.config,
                            0,
                        );
                    }
                    IPCEvent::RefreshIndex => {
                        let app_state = app_state.borrow();
                        let proxy = app_state.proxy.clone();
                        let plugins = app_state.plugins.clone();
                        thread::spawn(move || {
                            let res = || -> anyhow::Result<()> {
                                let config = Config::load()?;
                                proxy.send_event(AppEvent::ThreadEvent(
                                    ThreadEvent::UpdateConfig(config.clone()),
                                ))?;
                                for plugin in
                                    plugins.lock().unwrap().iter_mut().filter(|p| p.enabled())
                                {
                                    plugin.refresh(&config);
                                }
                                proxy.send_event(AppEvent::ThreadEvent(
                                    ThreadEvent::RefreshingIndexFinished,
                                ))?;
                                Ok(())
                            };

                            if let Err(e) = res() {
                                tracing::error!("{e}");
                            }
                        });
                    }
                    IPCEvent::HideMainWindow => {
                        hide_main_window(&app_state.borrow(), true);
                    }
                    _ => {}
                };
            }

            // handle ipc events from other windows
            AppEvent::Ipc(_window_id, _event) => {}

            AppEvent::ThreadEvent(event) => match event {
                ThreadEvent::RefreshingIndexFinished => {
                    emit_event(
                        &app_state.borrow().main_window,
                        IPCEvent::RefreshingIndexFinished,
                        &"",
                    );
                }
                ThreadEvent::UpdateConfig(c) => {
                    app_state.borrow_mut().config = c.clone();
                }
            },

            AppEvent::WebviewEvent { event, window_id } => match event {
                #[cfg(windows)]
                WebviewEvent::Focus(focus) => {
                    let app_state = app_state.borrow();
                    let main_window = app_state.main_window.window();
                    // hide main window when the webview loses focus
                    if *window_id == main_window.id() && !focus {
                        main_window.set_visible(false);
                    }
                }
            },
        },
        Event::WindowEvent { event, .. } => {
            match event {
                #[cfg(not(target_os = "windows"))]
                WindowEvent::Focused(focus) => {
                    let app_state = app_state.borrow();
                    let main_window = app_state.main_window.window();
                    // hide main window when it loses focus
                    #[cfg(any(target_os = "linux", target_os = "macos"))]
                    if *window_id == main_window.id() && !focus {
                        main_window.set_visible(false);
                    }
                }
                _ => {}
            }
        }
        _ => {}
    }
    Ok(())
}

#[tracing::instrument]
fn run() -> anyhow::Result<()> {
    let config = Config::load()?;
    let plugins: Arc<Mutex<Vec<Box<dyn Plugin + Send + 'static>>>> = Arc::new(Mutex::new(vec![
        AppLauncherPlugin::new(&config)?,
        DirectoryIndexerPlugin::new(&config)?,
    ]));

    let plugins_c = plugins.clone();
    let config_c = config.clone();
    thread::spawn(move || {
        for plugin in plugins_c.lock().unwrap().iter_mut().filter(|p| p.enabled()) {
            plugin.refresh(&config_c);
        }
    });

    let event_loop = EventLoop::<AppEvent>::with_user_event();
    event_loop.set_device_event_filter(DeviceEventFilter::Never);
    let main_window = create_main_window(&config, &event_loop)?;
    let main_window_id = main_window.window().id();
    let app_state = RefCell::new(AppState {
        config,
        main_window,
        #[cfg(windows)]
        previously_foreground_hwnd: 0,
        plugins,
        current_results: Default::default(),
        modifier_pressed: false,
        proxy: event_loop.create_proxy(),
    });

    event_loop.run(move |event, _event_loop, control_flow| {
        let mut _run = || -> anyhow::Result<()> {
            if let Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
                ..
            } = &event
            {
                if window_id == &main_window_id {
                    *control_flow = ControlFlow::Exit
                }
            }

            process_events(&event, &app_state)?;

            Ok(())
        };

        if let Err(e) = _run() {
            tracing::error!("{e}");
        }
    });
}

#[tracing::instrument]
fn main() -> anyhow::Result<()> {
    let appender = tracing_appender::rolling::never(&*KAL_DATA_DIR, "kal.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(appender);

    use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt};
    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish()
            .with(
                tracing_subscriber::fmt::Layer::default()
                    .with_writer(non_blocking)
                    .with_ansi(false),
            ),
    )
    .expect("failed to setup logger");

    run()?;

    Ok(())
}
