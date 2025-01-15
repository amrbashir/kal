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
use crate::plugin::PluginStore;
use crate::utils::thread;
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
        let plugin_store = crate::plugins::all(&config, &data_dir)?;
        {
            let config = config.clone();
            let mut plugin_store = plugin_store.clone();
            thread::spawn(move || plugin_store.refresh(&config));
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
