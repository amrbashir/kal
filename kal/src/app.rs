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
    #[cfg(windows)]
    SystemSettingsChanged,
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
        if let Err(e) = plugin_store.reload(&config) {
            tracing::error!("{e}");
        }

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
            IpcAction::Query => {
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

            IpcAction::Reload => {
                let old_hotkey = self.config.general.hotkey.clone();
                self.config = Config::load()?;

                self.plugin_store.reload(&self.config)?;

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

    #[cfg(windows)]
    fn listen_for_settings_change(&self, event_loop: &dyn ActiveEventLoop) {
        use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
        use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
        use winit::platform::windows::ActiveEventLoopExtWindows;
        use wry::raw_window_handle::RawWindowHandle;

        let Ok(handle) = event_loop.rwh_06_window_handle().window_handle() else {
            return;
        };

        let RawWindowHandle::Win32(handle) = handle.as_raw() else {
            return;
        };

        let hwnd = HWND(handle.hwnd.get() as _);

        let userdata = Box::new((self.sender.clone(), self.event_loop_proxy.clone()));
        let userdata = Box::into_raw(userdata);

        let _ = unsafe { SetWindowSubclass(hwnd, Some(event_loop_subclass), 0, userdata as _) };

        unsafe extern "system" fn event_loop_subclass(
            hwnd: HWND,
            umsg: u32,
            wparam: WPARAM,
            lparam: LPARAM,
            _: usize,
            userdata: usize,
        ) -> LRESULT {
            if umsg == WM_SETTINGCHANGE {
                let userdata = userdata as *const (mpsc::Sender<AppEvent>, EventLoopProxy);
                let (sender, proxy) = &*userdata;

                match sender.send(AppEvent::SystemSettingsChanged) {
                    Ok(_) => proxy.wake_up(),
                    Err(e) => tracing::error!("{e}"),
                }
            }

            DefSubclassProc(hwnd, umsg, wparam, lparam)
        }
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
                if let Err(e) = tx.send(res) {
                    tracing::error!("Failed to send ipc response: {e}");
                }
            }

            #[cfg(windows)]
            AppEvent::SystemSettingsChanged => {
                if let Some(system_accent_color) = crate::utils::system_accent_color() {
                    for window in self.windows.values() {
                        window.emit(IpcEvent::UpdateSystemAccentColor, &system_accent_color)?;
                    }
                }
            }
        }

        Ok(())
    }
}

impl ApplicationHandler for App {
    #[cfg(windows)]
    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, cause: winit::event::StartCause) {
        if cause == winit::event::StartCause::Init {
            self.listen_for_settings_change(event_loop);
        }
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.create_main_window(event_loop)
            .expect("Failed to create main window");
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        while let Ok(action) = self.receiver.try_recv() {
            if let Err(e) = self.app_event(event_loop, action) {
                tracing::error!("Error while processing `AppEvent`: {e}");
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            #[cfg(windows)]
            WindowEvent::RedrawRequested => {
                for window in self.windows.values_mut() {
                    if let Err(e) = window.clear_window_surface() {
                        tracing::error!("{e}");
                    }
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
