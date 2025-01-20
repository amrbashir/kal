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
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;

use crate::config::Config;
use crate::ipc::{self, IpcEvent};
use crate::plugin_store::PluginStore;
use crate::result_item::ResultItem;
use crate::webview_window::WebViewWindow;

pub enum AppEvent {
    HotKey(global_hotkey::GlobalHotKeyEvent),
    Ipc {
        label: String,
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

    pub results: Vec<ResultItem>,
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

            results: Vec::new(),
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

    #[cfg(windows)]
    fn listen_for_settings_change(&self, event_loop: &dyn ActiveEventLoop) {
        use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
        use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
        use winit::platform::windows::ActiveEventLoopExtWindows;

        let hwnd = HWND(event_loop.target_window_hwnd() as _);

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
                    Err(e) => {
                        tracing::error!("Failed to send `AppEvent::SystemSettingsChanged`:{e}")
                    }
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

            AppEvent::Ipc { label, request, tx } => {
                let res = if let Some(window) = self.windows.get(label.as_str()) {
                    if let Some(handler) = window.ipc_handler {
                        handler(self, request)
                    } else {
                        ipc::response::error_owned(format!(
                            "window with label {label} doesn't have an IPC handler"
                        ))
                    }
                } else {
                    ipc::response::error_owned(format!("Couldn't find window with label: {label}"))
                };

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
                        tracing::error!("Failed to clear window surface: {e}");
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
                if window_id == main_window.id() && !focus {
                    main_window.set_visible(false);
                }
            }

            _ => {}
        }
    }
}
