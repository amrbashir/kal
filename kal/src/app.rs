use std::borrow::Cow;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc;

use fuzzy_matcher::skim::SkimMatcherV2;
use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
#[cfg(windows)]
use windows::Win32::Foundation::HWND;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::*;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;
use wry::http::{Request, Response};

use crate::config::Config;
use crate::ipc::{response, IpcAction, IpcEvent};
use crate::plugin::PluginStore;
use crate::webview_window::WebViewWindow;

pub enum AppEvent {
    HotKey(global_hotkey::GlobalHotKeyEvent),
    Ipc {
        request: wry::http::Request<Vec<u8>>,
        tx: mpsc::SyncSender<anyhow::Result<wry::http::Response<Cow<'static, [u8]>>>>,
    },
}

pub struct App {
    pub event_loop_proxy: EventLoopProxy,
    pub sender: mpsc::Sender<AppEvent>,
    pub receiver: mpsc::Receiver<AppEvent>,

    #[allow(unused)]
    pub global_hotkey_manager: GlobalHotKeyManager,

    pub config: Config,

    pub windows: HashMap<&'static str, WebViewWindow>,

    pub plugin_store: PluginStore,
    pub fuzzy_matcher: SkimMatcherV2,

    #[cfg(windows)]
    pub previously_foreground_hwnd: HWND,
}

impl App {
    pub fn new(data_dir: PathBuf, event_loop_proxy: EventLoopProxy) -> anyhow::Result<Self> {
        let (sender, receiver) = mpsc::channel();

        let config = Config::load()?;

        let mut plugin_store = crate::plugins::all(&config, &data_dir)?;
        let _ = plugin_store
            .refresh(&config)
            .inspect_err(|e| tracing::error!("{e}"));

        let global_hotkey_manager = GlobalHotKeyManager::new()?;
        global_hotkey_manager.register(HotKey::try_from(config.general.hotkey.as_str())?)?;
        {
            let event_loop_proxy = event_loop_proxy.clone();
            let sender = sender.clone();
            GlobalHotKeyEvent::set_event_handler(Some(move |e| {
                event_loop_proxy.wake_up();
                if let Err(e) = sender.send(AppEvent::HotKey(e)) {
                    tracing::error!("Failed to send `AppEvent::HotKey`: {e}");
                }
            }));
        }

        Ok(Self {
            event_loop_proxy,
            sender,
            receiver,

            global_hotkey_manager,

            config,

            windows: HashMap::default(),

            plugin_store,
            fuzzy_matcher: SkimMatcherV2::default(),

            #[cfg(windows)]
            previously_foreground_hwnd: HWND::default(),
        })
    }

    #[cfg(windows)]
    pub fn store_foreground_hwnd(&mut self) {
        self.previously_foreground_hwnd = unsafe { GetForegroundWindow() };
    }

    #[cfg(windows)]
    pub fn restore_prev_foreground_hwnd(&self) {
        let _ = unsafe { SetForegroundWindow(self.previously_foreground_hwnd) };
    }

    fn resize_main_window_for_items(&self, count: usize) {
        let main_window = self.main_window();

        let items_height = if count == 0 {
            0
        } else {
            let count = std::cmp::min(count, self.config.appearance.max_items as usize) as u32;
            let item_height = self.config.appearance.item_height + self.config.appearance.item_gap;
            self.config.appearance.input_items_gap + count * item_height
        };

        let height = self.config.appearance.input_height + items_height + Self::MAGIC_BORDERS;

        let size = LogicalSize::new(self.config.appearance.window_width, height);
        let _ = main_window.window().request_surface_size(size.into());
    }

    pub fn ipc_event<'a>(
        &mut self,
        request: Request<Vec<u8>>,
    ) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
        let action: IpcAction = request.uri().path()[1..].try_into()?;

        match action {
            IpcAction::Search => {
                let body = request.body();
                let query = std::str::from_utf8(body)?;

                let mut results = Vec::new();

                self.plugin_store
                    .results(query, &self.fuzzy_matcher, &mut results)?;

                // sort results in reverse so higher scores are first
                results.sort_by(|a, b| b.score.cmp(&a.score));

                let min = std::cmp::min(self.config.general.max_results, results.len());
                let final_results = &results[..min];

                let json = response::json(&final_results);

                self.resize_main_window_for_items(min);

                return json;
            }

            IpcAction::ClearResults => self.resize_main_window_for_items(0),

            IpcAction::Execute => {
                let payload = request.body();
                let elevated: bool = payload[0] == 1;
                let id = std::str::from_utf8(&payload[1..])?;
                self.plugin_store.execute(id, elevated)?;
                self.hide_main_window(false);
            }

            IpcAction::ShowItemInDir => {
                let id = std::str::from_utf8(request.body())?;
                self.plugin_store.show_item_in_dir(id)?;
                self.hide_main_window(false);
            }

            IpcAction::RefreshIndex => {
                let old_hotkey = self.config.general.hotkey.clone();
                self.config = Config::load()?;

                self.plugin_store.refresh(&self.config)?;

                let main_window = self.main_window();
                main_window.emit(IpcEvent::UpdateConfig, &self.config)?;

                let old_hotkey = HotKey::try_from(old_hotkey.as_str())?;
                let new_hotkey = HotKey::try_from(self.config.general.hotkey.as_str())?;
                if old_hotkey != new_hotkey {
                    self.global_hotkey_manager.unregister(old_hotkey)?;
                    self.global_hotkey_manager.register(new_hotkey)?;
                }
            }

            IpcAction::HideMainWindow => {
                self.hide_main_window(true);
            }
        }

        response::empty()
    }

    fn app_event(
        &mut self,
        _event_loop: &dyn ActiveEventLoop,
        event: AppEvent,
    ) -> anyhow::Result<()> {
        match event {
            AppEvent::HotKey(e) if e.state == HotKeyState::Pressed => {
                if self.main_window().window().is_visible().unwrap_or_default() {
                    self.hide_main_window(true);
                } else {
                    self.show_main_window()?;
                }
            }

            AppEvent::HotKey(_) => {}

            AppEvent::Ipc { request, tx } => {
                let res = self.ipc_event(request);
                let _ = tx
                    .send(res)
                    .inspect_err(|e| tracing::error!("Failed to send ipc response: {e}"));
            }
        }

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.create_main_window(event_loop)
            .expect("Failed to create main window");
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Ok(action) = self.receiver.try_recv() {
            let _ = self
                .app_event(event_loop, action)
                .inspect_err(|e| tracing::error!("Error while processing `AppEvent`: {e}"));
        }
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::RedrawRequested => {
                for window in self.windows.values_mut() {
                    let _ = window
                        .clear_window_surface()
                        .inspect_err(|e| tracing::error!("{e}"));
                }
            }

            WindowEvent::CloseRequested => {
                if window_id == self.main_window().id() {
                    event_loop.exit();
                }
            }

            #[cfg(not(debug_assertions))]
            WindowEvent::Focused(focus) => {
                let main_window = self.main_window().window();
                // hide main window when it loses focus
                if *window_id == main_window.id() && !focus {
                    main_window.set_visible(false);
                }
            }

            _ => {}
        }
    }
}
