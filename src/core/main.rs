use std::{cell::RefCell, path::PathBuf};

use anyhow::Context;
use fuzzy_matcher::skim::SkimMatcherV2;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use plugin::PluginStore;
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt};

#[cfg(windows)]
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};

#[cfg(target_os = "linux")]
use tao::platform::linux::WindowExtUnix;
#[cfg(windows)]
use tao::platform::windows::WindowExtWindows;

use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, DeviceEventFilter, EventLoop, EventLoopBuilder, EventLoopProxy},
    window::WindowAttributes,
};
use wry::WebViewAttributes;

use crate::{
    common::IPCEvent,
    config::Config,
    event::{emit_event, AppEvent, ThreadEvent, KAL_IPC_INIT_SCRIPT},
    utils::thread,
    webview_window::WebviewWindow,
};

#[cfg(not(debug_assertions))]
use crate::event::WebviewEvent;

#[path = "../common/mod.rs"]
mod common;
mod config;
mod event;
mod plugin;
mod plugins;
mod protocol;
mod utils;
mod vibrancy;
mod webview_window;

#[cfg(not(debug_assertions))]
#[derive(rust_embed::RustEmbed)]
#[folder = "dist"]
pub(crate) struct EmbededAssets;

#[tracing::instrument]
fn create_main_window(
    config: &Config,
    event_loop: &EventLoop<AppEvent>,
) -> anyhow::Result<WebviewWindow> {
    let (m_size, m_pos) = {
        let monitor = event_loop
            .primary_monitor()
            .with_context(|| "Failed to get primary monitor")?;
        (
            monitor.size().to_logical::<u32>(monitor.scale_factor()),
            monitor.position().to_logical::<u32>(monitor.scale_factor()),
        )
    };

    #[cfg(debug_assertions)]
    let url = "http://localhost:9010";
    #[cfg(not(debug_assertions))]
    let url = "kal://localhost";

    let serialize_options = serialize_to_javascript::Options::default();
    let mut initialization_scripts = vec![
        KAL_IPC_INIT_SCRIPT.to_string(),
        format!(
            "(function () {{ window.KAL.config = {}; }})()",
            serialize_to_javascript::Serialized::new(
                &serde_json::value::to_raw_value(&config).unwrap_or_default(),
                &serialize_options,
            ),
        ),
    ];

    if let Some(file) = &config.appearance.custom_css_file {
        let contents = std::fs::read_to_string(file)?;
        initialization_scripts.push(format!(
            r#"(function () {{
                  window.addEventListener("DOMContentLoaded", () => {{
                    const style = document.createElement("style");
                    style.textContent = {};
                    const head = document.head ?? document.querySelector('head') ?? document.body;
                    head.appendChild(style)
                  }})
                }})()"#,
            serialize_to_javascript::Serialized::new(
                &serde_json::value::to_raw_value(&contents).unwrap_or_default(),
                &serialize_options,
            ),
        ));
    }

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
                    m_pos.x + (m_size.width / 2 - config.appearance.window_width / 2),
                    m_pos.y + (m_size.height / 4),
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
            initialization_scripts,
            devtools: cfg!(debug_assertions),
            ..Default::default()
        },
        event_loop,
    )?;

    #[cfg(any(windows, target_os = "linux"))]
    main_window.window().set_skip_taskbar(true);

    if let Some(vibrancy) = &config.appearance.vibrancy {
        vibrancy.apply(&main_window)?;
    }

    Ok(main_window)
}

// Saves the current foreground window then shows the main window
#[tracing::instrument]
fn show_main_window(app_state: &mut AppState<AppEvent>) -> anyhow::Result<()> {
    #[cfg(windows)]
    app_state.store_foreground_hwnd();

    let main_window = &app_state.main_window;
    main_window.window().set_visible(true);
    main_window.window().set_focus();
    emit_event(main_window.webview(), IPCEvent::FocusInput, ())
}

/// Hides the main window and restores focus to the previous foreground window if needed
#[tracing::instrument]
fn hide_main_window(app_state: &AppState<AppEvent>, #[allow(unused)] restore_focus: bool) {
    app_state.main_window.window().set_visible(false);

    #[cfg(windows)]
    if restore_focus {
        app_state.restore_prev_foreground_hwnd();
    }
}

// Resizes the main window based on the number of current results and user config
#[tracing::instrument]
fn resize_main_window_for_results(main_window: &WebviewWindow, config: &Config, count: usize) {
    let count = count as u32;
    let gaps = count.saturating_sub(1);

    let results_height = if count == 0 {
        0
    } else {
        std::cmp::min(
            count * config.appearance.results_row_height + 16  /* padding */ + gaps * 4 /* gap */ + 1, /* divider */
            config.appearance.results_height,
        )
    };

    let height = results_height + config.appearance.input_height;

    main_window
        .window()
        .set_inner_size(LogicalSize::new(config.appearance.window_width, height));
}

struct AppState<T: 'static> {
    config: Config,

    main_window: WebviewWindow,
    #[cfg(windows)]
    previously_foreground_hwnd: HWND,

    plugin_store: PluginStore,
    fuzzy_matcher: SkimMatcherV2,

    proxy: EventLoopProxy<T>,

    data_dir: PathBuf,
    config_file: PathBuf,
}

impl<T: 'static> std::fmt::Debug for AppState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppState")
            .field("config", &self.config)
            .field("main_window", &self.main_window)
            .field(
                "previously_foreground_hwnd",
                &self.previously_foreground_hwnd,
            )
            .field("plugin_store", &self.plugin_store)
            .field("proxy", &self.proxy)
            .field("data_dir", &self.data_dir)
            .field("config_file", &self.config_file)
            .finish()
    }
}

impl<T: 'static> AppState<T> {
    fn new(
        config: Config,
        main_window: WebviewWindow,
        plugin_store: PluginStore,
        proxy: EventLoopProxy<T>,
        data_dir: PathBuf,
        config_file: PathBuf,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            main_window,
            #[cfg(windows)]
            previously_foreground_hwnd: HWND::default(),
            plugin_store,
            proxy,
            fuzzy_matcher: SkimMatcherV2::default(),
            data_dir,
            config_file,
        })
    }

    #[cfg(windows)]
    fn store_foreground_hwnd(&mut self) {
        self.previously_foreground_hwnd = unsafe { GetForegroundWindow() };
    }

    #[cfg(windows)]
    fn restore_prev_foreground_hwnd(&self) {
        unsafe { SetForegroundWindow(self.previously_foreground_hwnd) };
    }
}

#[tracing::instrument]
fn process_ipc_events(
    app_state: &RefCell<AppState<AppEvent>>,
    request: &str,
) -> anyhow::Result<()> {
    let (event, payload) = request
        .split_once("::")
        .with_context(|| "Invalid IPC call syntax")?;
    let event: IPCEvent = event.into();

    match event {
        IPCEvent::Search => {
            let app_state = app_state.borrow();

            let query = serde_json::from_str::<&str>(payload)?;

            let mut results = Vec::new();

            let store = app_state.plugin_store.lock();
            for plugin in store.plugins() {
                results.extend(plugin.results(query, &app_state.fuzzy_matcher)?);
            }

            // sort results in reverse so higher scores are first
            results.sort_by(|a, b| b.score.cmp(&a.score));

            let min = std::cmp::min(app_state.config.general.max_search_results, results.len());
            let final_results = &results[..min];

            emit_event(
                app_state.main_window.webview(),
                IPCEvent::Results,
                final_results,
            )?;
            resize_main_window_for_results(&app_state.main_window, &app_state.config, min);
        }

        IPCEvent::Execute => {
            let app_state = app_state.borrow();
            let (id, elevated) = serde_json::from_str::<(&str, bool)>(payload)?;
            app_state.plugin_store.execute(id, elevated)?;
            hide_main_window(&app_state, false);
        }

        IPCEvent::OpenLocation => {
            let app_state = app_state.borrow();
            let id = serde_json::from_str::<&str>(payload)?;
            app_state.plugin_store.reveal_in_dir(id)?;
            hide_main_window(&app_state, false);
        }

        IPCEvent::ClearResults => {
            let app_state = app_state.borrow();
            resize_main_window_for_results(&app_state.main_window, &app_state.config, 0);
        }

        IPCEvent::RefreshIndex => {
            let app_state = app_state.borrow();
            let proxy = app_state.proxy.clone();
            let mut plugin_store = app_state.plugin_store.clone();
            let config_file = app_state.config_file.clone();
            thread::spawn(move || {
                let config = Config::load_from_path(config_file)?;
                plugin_store.refresh(&config)?;
                proxy.send_event(AppEvent::ThreadEvent(ThreadEvent::RefreshingIndexFinished))?;
                proxy.send_event(AppEvent::ThreadEvent(ThreadEvent::UpdateConfig(config)))?;
                Ok(())
            });
        }

        IPCEvent::HideMainWindow => {
            hide_main_window(&app_state.borrow(), true);
        }

        _ => {}
    }

    Ok(())
}

#[tracing::instrument]
fn process_events(
    event: &Event<AppEvent>,
    app_state: &RefCell<AppState<AppEvent>>,
) -> anyhow::Result<()> {
    match event {
        // clear window surface on windows for transparent windows
        #[cfg(windows)]
        Event::NewEvents(StartCause::Init) | Event::RedrawRequested(_) => {
            let mut app_state = app_state.borrow_mut();
            app_state.main_window.clear_window_surface()?;
        }

        #[cfg(all(not(windows), not(debug_assertions)))]
        Event::WindowEvent {
            event, window_id, ..
        } => {
            match event {
                WindowEvent::Focused(focus) => {
                    let app_state = app_state.borrow();
                    let main_window = app_state.main_window.window();
                    // hide main window when it loses focus
                    if *window_id == main_window.id() && !focus {
                        main_window.set_visible(false);
                    }
                }
                _ => {}
            }
        }

        Event::UserEvent(event) => match event {
            #[cfg(all(windows, not(debug_assertions)))]
            AppEvent::WebviewEvent { event, window_id } => match event {
                WebviewEvent::Focus(focus) => {
                    let app_state = app_state.borrow();
                    let main_window = &app_state.main_window.window;
                    // hide main window when the webview loses focus
                    if *window_id == main_window.id() && !focus {
                        main_window.set_visible(false);
                    }
                }
            },

            AppEvent::HotKey(e) if e.state == HotKeyState::Pressed => {
                let mut app_state = app_state.borrow_mut();
                if app_state.main_window.window().is_visible() {
                    hide_main_window(&app_state, true);
                } else {
                    show_main_window(&mut app_state)?;
                }
            }

            AppEvent::Ipc(_window_id, request) => {
                process_ipc_events(app_state, request)?;
            }

            AppEvent::ThreadEvent(event) => match event {
                ThreadEvent::RefreshingIndexFinished => {
                    emit_event(
                        app_state.borrow().main_window.webview(),
                        IPCEvent::RefreshingIndexFinished,
                        (),
                    )?;
                }
                ThreadEvent::UpdateConfig(c) => {
                    app_state.borrow_mut().config = c.clone();
                }
            },

            _ => {}
        },

        _ => {}
    }
    Ok(())
}

#[tracing::instrument]
fn run(data_dir: PathBuf) -> anyhow::Result<()> {
    #[cfg(debug_assertions)]
    let config_file = std::env::current_dir()
        .context("Failed to get current directory path")?
        .join("kal.toml");
    #[cfg(not(debug_assertions))]
    let config_file = dirs::home_dir()
        .context("Failed to get $home_dir path")?
        .join(".config")
        .join("kal.toml");
    let config = Config::load_from_path(&config_file)?;
    let plugin_store = plugins::all(&config, &data_dir)?;

    let config_c = config.clone();
    let mut plugin_store_c = plugin_store.clone();
    thread::spawn(move || plugin_store_c.refresh(&config_c));

    let event_loop = EventLoopBuilder::<AppEvent>::with_user_event().build();
    event_loop.set_device_event_filter(DeviceEventFilter::Always);

    let gh_manager = GlobalHotKeyManager::new()?;
    gh_manager.register(HotKey::try_from(config.general.hotkey.as_str())?)?;
    let proxy = event_loop.create_proxy();
    GlobalHotKeyEvent::set_event_handler(Some(move |e| {
        if let Err(e) = proxy.send_event(AppEvent::HotKey(e)) {
            tracing::error!("{e}");
        }
    }));

    let main_window = create_main_window(&config, &event_loop)?;
    let main_window_id = main_window.window().id();

    let app_state = AppState::new(
        config,
        main_window,
        plugin_store,
        event_loop.create_proxy(),
        data_dir,
        config_file,
    )?;
    let app_state = RefCell::new(app_state);

    event_loop.run(move |event, _event_loop, control_flow| {
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

        if let Err(e) = process_events(&event, &app_state) {
            tracing::error!("{e}");
        }
    });
}

#[tracing::instrument]
fn main() -> anyhow::Result<()> {
    let data_dir = dirs::data_local_dir()
        .context("Failed to get $data_local_dir path")?
        .join("kal");

    let appender = tracing_appender::rolling::never(&data_dir, "kal.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(appender);

    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(non_blocking)
        .with_ansi(false);

    tracing::subscriber::set_global_default(
        tracing_subscriber::fmt::Subscriber::builder()
            .with_max_level(LevelFilter::TRACE)
            .finish()
            .with(layer),
    )?;

    run(data_dir).inspect_err(|e| tracing::error!("{e}"))
}
