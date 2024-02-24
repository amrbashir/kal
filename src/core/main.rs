use std::{
    cell::RefCell,
    path::PathBuf,
    sync::{Arc, Mutex, MutexGuard},
};

use anyhow::Context;
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_sort::FuzzySort;
use global_hotkey::{hotkey::HotKey, GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use once_cell::sync::Lazy;
use tao::{
    dpi::{LogicalPosition, LogicalSize},
    event::{Event, StartCause, WindowEvent},
    event_loop::{ControlFlow, DeviceEventFilter, EventLoop, EventLoopBuilder, EventLoopProxy},
    window::WindowAttributes,
};
use tracing_subscriber::{filter::LevelFilter, layer::SubscriberExt};
use windows::Win32::Foundation::HWND;
use wry::WebViewAttributes;

use crate::{
    common::{IPCEvent, SearchResultItem},
    config::Config,
    event::{emit_event, AppEvent, ThreadEvent, KAL_IPC_INIT_SCRIPT},
    plugin::Plugin,
    utils::thread,
    webview_window::WebviewWindow,
};

#[cfg(not(debug_assertions))]
use crate::event::WebviewEvent;

#[path = "../common/mod.rs"]
mod common;
mod config;
mod event;
mod fuzzy_sort;
mod plugin;
mod plugins;
mod protocols;
mod utils;
mod vibrancy;
mod webview_window;

static KAL_DATA_DIR: Lazy<PathBuf> = Lazy::new(|| {
    dirs::data_local_dir()
        .expect("Failed to get $data_local_dir path")
        .join("kal")
});
static CONFIG_FILE: Lazy<PathBuf> = Lazy::new(|| {
    #[cfg(debug_assertions)]
    return std::env::current_dir()
        .expect("Failed to get current directory path")
        .join("kal.toml");
    #[cfg(not(debug_assertions))]
    dirs::home_dir()
        .expect("Failed to get $home_dir path")
        .join(".config")
        .join("kal.toml")
});
static TEMP_DIR: Lazy<PathBuf> = Lazy::new(std::env::temp_dir);

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
        if let Some(file) = CONFIG_FILE.parent().map(|p| p.join(file)) {
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

    #[cfg(all(not(debug_assertions), any(windows, target_os = "linux")))]
    {
        #[cfg(target_os = "linux")]
        use tao::platform::linux::WindowExtWindows;
        #[cfg(windows)]
        use tao::platform::windows::WindowExtWindows;
        main_window.window.set_skip_taskbar(true);
    }

    if let Some(vibrancy) = &config.appearance.vibrancy {
        vibrancy.apply(&main_window)?;
    }

    Ok(main_window)
}

// Saves the current foreground window then shows the main window
#[tracing::instrument]
fn show_main_window(app_state: &mut AppState<AppEvent>) -> anyhow::Result<()> {
    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
        app_state.previously_foreground_hwnd = unsafe { GetForegroundWindow() };
    }

    let main_window = &app_state.main_window;
    main_window.window.set_visible(true);
    main_window.window.set_focus();
    emit_event(main_window, IPCEvent::FocusInput, ())
}

/// Hides the main window and restores focus to the previous foreground window if needed
#[tracing::instrument]
fn hide_main_window<T>(app_state: &AppState<T>, #[allow(unused)] restore_focus: bool) {
    app_state.main_window.window.set_visible(false);

    #[cfg(windows)]
    {
        use windows::Win32::UI::WindowsAndMessaging::SetForegroundWindow;

        if restore_focus {
            unsafe { SetForegroundWindow(app_state.previously_foreground_hwnd) };
        }
    }
}

// Resizes the main window based on the number of current results and user config
#[tracing::instrument]
fn resize_main_window_for_results(main_window: &WebviewWindow, config: &Config, count: usize) {
    main_window.window.set_inner_size(LogicalSize::new(
        config.appearance.window_width,
        std::cmp::min(
            count as u32 * config.appearance.results_item_height,
            config.appearance.results_height,
        ) + config.appearance.input_height,
    ));
}

struct AppState<T: 'static> {
    config: Config,
    main_window: WebviewWindow,
    #[cfg(windows)]
    previously_foreground_hwnd: windows::Win32::Foundation::HWND,
    plugins: Arc<Mutex<Vec<Box<dyn Plugin + Send + 'static>>>>,
    current_results: Vec<SearchResultItem>,
    modifier_pressed: bool,
    proxy: EventLoopProxy<T>,
    fuzzy_matcher: SkimMatcherV2,
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
            .field("plugins", &self.plugins)
            .field("current_results", &self.current_results)
            .field("modifier_pressed", &self.modifier_pressed)
            .field("proxy", &self.proxy)
            .finish()
    }
}

impl<T: 'static> AppState<T> {
    fn new(
        config: Config,
        main_window: WebviewWindow,
        plugins: Arc<Mutex<Vec<Box<dyn Plugin + Send + 'static>>>>,
        proxy: EventLoopProxy<T>,
    ) -> Self {
        Self {
            config,
            main_window,
            #[cfg(windows)]
            previously_foreground_hwnd: HWND::default(),
            plugins,
            current_results: Default::default(),
            modifier_pressed: false,
            proxy,
            fuzzy_matcher: SkimMatcherV2::default(),
        }
    }

    fn plugins(&self) -> MutexGuard<'_, Vec<Box<dyn Plugin + Send>>> {
        self.plugins.lock().unwrap()
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
            let mut app_state = app_state.borrow_mut();

            let query = serde_json::from_str::<&str>(payload)?;

            let mut results = Vec::new();
            {
                let plugins = app_state.plugins();
                let enabled_plugins = plugins.iter().filter(|p| p.enabled());
                for plugin in enabled_plugins {
                    let plugin_results = plugin.results(query)?;
                    results.extend_from_slice(plugin_results);
                }
            }
            results.fuzzy_sort(query, &app_state.fuzzy_matcher);

            let min = std::cmp::min(app_state.config.general.max_search_results, results.len());
            let final_results = &results[..=min];

            emit_event(&app_state.main_window, IPCEvent::Results, final_results)?;
            resize_main_window_for_results(&app_state.main_window, &app_state.config, min);

            app_state.current_results = results;
        }

        IPCEvent::Execute => {
            let app_state = app_state.borrow();

            let (index, elevated) = serde_json::from_str::<(usize, bool)>(payload)?;

            let item = &app_state.current_results[index];
            app_state
                .plugins()
                .iter()
                .find(|p| p.name() == item.plugin_name && p.enabled())
                .ok_or_else(|| anyhow::anyhow!("Failed to find  {}!", item.plugin_name))?
                .execute(item, elevated)?;

            hide_main_window(&app_state, false);
        }

        IPCEvent::OpenLocation => {
            let app_state = app_state.borrow();

            let index = serde_json::from_str::<usize>(payload)?;

            let item = &app_state.current_results[index];
            app_state
                .plugins()
                .iter()
                .find(|p| p.name() == item.plugin_name)
                .ok_or_else(|| anyhow::anyhow!("Failed to find  {}!", item.plugin_name))?
                .open_location(item)?;

            hide_main_window(&app_state, false);
        }

        IPCEvent::ClearResults => {
            let mut app_state = app_state.borrow_mut();
            resize_main_window_for_results(&app_state.main_window, &app_state.config, 0);
            app_state.current_results.clear();
        }

        IPCEvent::RefreshIndex => {
            let app_state = app_state.borrow();
            let proxy = app_state.proxy.clone();
            let plugins = app_state.plugins.clone();
            thread::spawn(move || {
                let config = Config::load()?;
                for plugin in plugins.lock().unwrap().iter_mut() {
                    plugin.refresh(&config)?;
                }
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
            app_state.main_window.clear_window_surface();
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
                let window = &app_state.main_window.window;
                if window.is_visible() {
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
                        &app_state.borrow().main_window,
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
fn run() -> anyhow::Result<()> {
    let config = Config::load()?;
    let plugins = Arc::new(Mutex::new(plugins::all(&config)?));

    let config_c = config.clone();
    let plugins_c = plugins.clone();
    thread::spawn(move || {
        for plugin in plugins_c.lock().unwrap().iter_mut().filter(|p| p.enabled()) {
            plugin.refresh(&config_c)?;
        }
        Ok(())
    });

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
    let main_window_id = main_window.window.id();

    let app_state = AppState::new(config, main_window, plugins, event_loop.create_proxy());
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
    let appender = tracing_appender::rolling::never(&*KAL_DATA_DIR, "kal.log");
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

    run().inspect_err(|e| tracing::error!("{e}"))
}
