use std::borrow::Cow;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

use anyhow::Context;
use fuzzy_matcher::skim::SkimMatcherV2;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use plugin::PluginStore;
use strum::{AsRefStr, EnumString};
use tao::dpi::LogicalSize;
use tao::event::{Event, StartCause, WindowEvent};
use tao::event_loop::{
    ControlFlow, DeviceEventFilter, EventLoop, EventLoopBuilder, EventLoopProxy,
};
use tracing_subscriber::filter::LevelFilter;
use tracing_subscriber::layer::SubscriberExt;
use windowing::ipc;
#[cfg(windows)]
use windows::Win32::{Foundation::*, UI::WindowsAndMessaging::*};
use wry::http::{Request, Response};

use crate::config::Config;
use crate::utils::thread;
use crate::windowing::webview_window::{WebViewWindow, WebViewWindowBuilder};

mod config;
mod icon;
mod plugin;
mod plugins;
mod search_result_item;
mod utils;
mod windowing;

#[derive(EnumString, AsRefStr)]
pub enum IpcAction {
    Search,
    ClearResults,
    Execute,
    ShowItemInDir,
    RefreshIndex,
    HideMainWindow,
}

#[derive(EnumString, AsRefStr)]
pub enum IpcEvent {
    FocusInput,
}

#[derive(Debug)]
#[non_exhaustive]
#[cfg(all(windows, not(debug_assertions)))]
pub enum WebviewEvent {
    /// The webview gained or lost focus
    ///
    /// Currently, it is only used on Windows
    #[cfg(windows)]
    Focus(bool),
}

#[derive(Debug)]
#[non_exhaustive]
pub enum AppEvent {
    /// Describes an event from a [`WebView`]
    #[cfg(all(windows, not(debug_assertions)))]
    Webview {
        event: WebviewEvent,
        window_id: tao::window::WindowId,
    },
    /// A HotKey event.
    HotKey(global_hotkey::GlobalHotKeyEvent),
}

#[tracing::instrument(level = "trace")]
fn create_main_window(
    event_loop: &EventLoop<AppEvent>,
    app_state: &Rc<RefCell<AppState<AppEvent>>>,
) -> anyhow::Result<WebViewWindow> {
    #[cfg(debug_assertions)]
    let url = "http://localhost:9010";
    #[cfg(not(debug_assertions))]
    let url = "kal://localhost";

    let serialize_options = serialize_to_javascript::Options::default();

    let config = &app_state.borrow().config;

    let css_script = match config.appearance.custom_css_file {
        Some(ref file) => {
            let contents = std::fs::read_to_string(file)?;
            let script = format!(
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
            );
            Some(script)
        }
        None => None,
    };

    let builder = WebViewWindowBuilder::new()
        .url(url)
        .ipc(&process_ipc)
        .init_script(&format!(
            "(function () {{ window.KAL = {{}}; window.KAL.config = {}; }})()",
            serialize_to_javascript::Serialized::new(
                &serde_json::value::to_raw_value(&config).unwrap_or_default(),
                &serialize_options,
            ),
        ))
        .init_script_opt(css_script.as_deref())
        .inner_size(LogicalSize::new(
            config.appearance.window_width,
            config.appearance.input_height,
        ))
        .center(true)
        .decorations(false)
        .resizable(false)
        .visible(false)
        .vibrancy(config.appearance.vibrancy)
        .transparent(config.appearance.transparent)
        .skip_taskbar(cfg!(any(windows, target_os = "linux")))
        .devtools(true);

    let main_window = builder.build(event_loop, app_state)?;

    Ok(main_window)
}

// Saves the current foreground window then shows the main window
#[tracing::instrument(level = "trace")]
fn show_main_window(app_state: &mut AppState<AppEvent>) -> anyhow::Result<()> {
    #[cfg(windows)]
    app_state.store_foreground_hwnd();

    let main_window = &app_state.main_window();
    main_window.window().set_visible(true);
    main_window.window().set_focus();
    main_window.emit(IpcEvent::FocusInput, ())
}

/// Hides the main window and restores focus to the previous foreground window if needed
#[tracing::instrument(level = "trace")]
fn hide_main_window(app_state: &AppState<AppEvent>, #[allow(unused)] restore_focus: bool) {
    app_state.main_window().window().set_visible(false);

    #[cfg(windows)]
    if restore_focus {
        app_state.restore_prev_foreground_hwnd();
    }
}

// Resizes the main window based on the number of current results and user config
#[tracing::instrument(level = "trace")]
fn resize_main_window_for_results(main_window: &WebViewWindow, config: &Config, count: usize) {
    let count = count as u32;
    let gaps = count.saturating_sub(1);

    let results_height = if count == 0 {
        0
    } else {
        std::cmp::min(
            count * config.appearance.results_row_height
                + config.appearance.results_padding
                + gaps * config.appearance.results_row_gap
                + config.appearance.results_divier,
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

    main_window: Option<WebViewWindow>,
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
        plugin_store: PluginStore,
        proxy: EventLoopProxy<T>,
        data_dir: PathBuf,
        config_file: PathBuf,
    ) -> anyhow::Result<Self> {
        Ok(Self {
            config,
            main_window: None,
            #[cfg(windows)]
            previously_foreground_hwnd: HWND::default(),
            plugin_store,
            proxy,
            fuzzy_matcher: SkimMatcherV2::default(),
            data_dir,
            config_file,
        })
    }

    #[inline]
    fn main_window(&self) -> &WebViewWindow {
        self.main_window.as_ref().unwrap()
    }

    #[inline]
    fn main_window_mut(&mut self) -> &mut WebViewWindow {
        self.main_window.as_mut().unwrap()
    }

    #[cfg(windows)]
    fn store_foreground_hwnd(&mut self) {
        self.previously_foreground_hwnd = unsafe { GetForegroundWindow() };
    }

    #[cfg(windows)]
    fn restore_prev_foreground_hwnd(&self) {
        let _ = unsafe { SetForegroundWindow(self.previously_foreground_hwnd) };
    }
}

#[tracing::instrument(level = "trace")]
fn process_ipc<'a>(
    app_state: &Rc<RefCell<AppState<AppEvent>>>,
    request: Request<Vec<u8>>,
) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    let action: IpcAction = request.uri().path()[1..].try_into()?;

    match action {
        IpcAction::Search => {
            let app_state = app_state.borrow();

            let body = request.body();
            let query = std::str::from_utf8(body)?;

            let mut results = Vec::new();

            let mut store = app_state.plugin_store.lock();
            store.results(query, &app_state.fuzzy_matcher, &mut results)?;

            // sort results in reverse so higher scores are first
            results.sort_by(|a, b| b.score.cmp(&a.score));

            let min = std::cmp::min(app_state.config.general.max_search_results, results.len());
            let final_results = &results[..min];

            let main_window = app_state.main_window();
            resize_main_window_for_results(main_window, &app_state.config, min);
            return ipc::make_json_response(&final_results);
        }

        IpcAction::ClearResults => {
            let app_state = app_state.borrow();
            resize_main_window_for_results(app_state.main_window(), &app_state.config, 0);
        }

        IpcAction::Execute => {
            let mut app_state = app_state.borrow_mut();
            let payload = request.body();
            let elevated: bool = payload[0] == 1;
            let id = std::str::from_utf8(&payload[1..])?;
            app_state.plugin_store.execute(id, elevated)?;
            hide_main_window(&app_state, false);
        }

        IpcAction::ShowItemInDir => {
            let app_state = app_state.borrow();
            let id = std::str::from_utf8(request.body())?;
            app_state.plugin_store.show_item_in_dir(id)?;
            hide_main_window(&app_state, false);
        }

        IpcAction::RefreshIndex => {
            let mut app_state = app_state.borrow_mut();
            let config = Config::load_from_path(&app_state.config_file)?;
            app_state.plugin_store.refresh(&config)?;
        }

        IpcAction::HideMainWindow => {
            hide_main_window(&app_state.borrow(), true);
        }
    }

    ipc::empty_response()
}

#[tracing::instrument(level = "trace")]
fn process_events(
    app_state: &RefCell<AppState<AppEvent>>,
    event: &Event<AppEvent>,
) -> anyhow::Result<()> {
    match event {
        // clear window surface on windows for transparent windows
        #[cfg(windows)]
        Event::NewEvents(StartCause::Init) | Event::RedrawRequested(_) => {
            let mut app_state = app_state.borrow_mut();
            app_state.main_window_mut().clear_window_surface()?;
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
            AppEvent::Webview { event, window_id } => match event {
                WebviewEvent::Focus(focus) => {
                    let app_state = app_state.borrow();
                    let main_window = &app_state.main_window().window();
                    // hide main window when the webview loses focus
                    if *window_id == main_window.id() && !focus {
                        main_window.set_visible(false);
                    }
                }
            },

            AppEvent::HotKey(e) if e.state == HotKeyState::Pressed => {
                let mut app_state = app_state.borrow_mut();
                if app_state.main_window().window().is_visible() {
                    hide_main_window(&app_state, true);
                } else {
                    show_main_window(&mut app_state)?;
                }
            }

            _ => {}
        },

        _ => {}
    }
    Ok(())
}

#[tracing::instrument(level = "trace")]
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

    let app_state = AppState::new(
        config,
        plugin_store,
        event_loop.create_proxy(),
        data_dir,
        config_file,
    )?;

    let app_state = Rc::new(RefCell::new(app_state));

    let main_window = create_main_window(&event_loop, &app_state)?;
    let main_window_id = main_window.window().id();
    app_state.borrow_mut().main_window = Some(main_window);

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

        if let Err(e) = process_events(&app_state, &event) {
            tracing::error!("{e}");
        }
    });
}

#[tracing::instrument(level = "trace")]
fn main() -> anyhow::Result<()> {
    let data_dir = dirs::data_local_dir()
        .context("Failed to get $data_local_dir path")?
        .join("kal");

    let appender = tracing_appender::rolling::never(&data_dir, "kal.log");
    let (non_blocking, _guard) = tracing_appender::non_blocking(appender);

    let layer = tracing_subscriber::fmt::Layer::default()
        .with_writer(non_blocking)
        .with_ansi(false);
    let subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(LevelFilter::TRACE)
        .finish()
        .with(layer);
    tracing::subscriber::set_global_default(subscriber)?;

    run(data_dir).inspect_err(|e| tracing::error!("{e}"))
}
