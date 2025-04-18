use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{mpsc, Arc};

use global_hotkey::hotkey::HotKey;
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};
use kal_config::Config;
use tray_icon::menu::{Menu, MenuEvent, MenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder, TrayIconEvent};
#[cfg(windows)]
use windows::Win32::Foundation::*;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::*;
use winit::application::ApplicationHandler;
use winit::dpi::Size;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoopProxy};
use winit::window::WindowId;
use wry::WebContext;

use crate::icon;
use crate::ipc::IpcEvent;
use crate::main_window::MainWindowState;
use crate::webview_window::WebViewWindow;

#[derive(Debug)]
pub enum AppMessage {
    HotKey(GlobalHotKeyEvent),
    TrayIcon(TrayIconEvent),
    Menu(MenuEvent),
    #[cfg(all(not(debug_assertions), windows))]
    WebviewFocused(WindowId, bool),
    #[cfg(windows)]
    SystemSettingsChanged,
    RequestSufaceSize(Size),
    HideMainWindow(bool),
    MainWindowEmit(IpcEvent, serde_json::Value),
    ReRegisterHotKey(HotKey, HotKey),
}

pub struct App {
    pub event_loop_proxy: EventLoopProxy,
    pub sender: mpsc::Sender<AppMessage>,
    pub receiver: mpsc::Receiver<AppMessage>,

    pub config: Config,

    pub global_hotkey_manager: GlobalHotKeyManager,

    pub windows: HashMap<&'static str, WebViewWindow>,

    #[cfg(windows)]
    pub previously_foreground_hwnd: HWND,

    pub icon_service: Arc<icon::Service>,

    #[allow(unused)]
    pub tray_icon: TrayIcon,

    pub web_context: wry::WebContext,
}

impl App {
    pub fn new(kal_data_dir: PathBuf, event_loop_proxy: EventLoopProxy) -> anyhow::Result<Self> {
        let (sender, receiver) = mpsc::channel();

        let config = Config::load_with_fallback();

        let global_hotkey_manager = GlobalHotKeyManager::new()?;
        global_hotkey_manager.register(HotKey::try_from(config.general.hotkey.as_str())?)?;

        let event_loop_proxy_ = event_loop_proxy.clone();
        let sender_ = sender.clone();
        GlobalHotKeyEvent::set_event_handler(Some(move |e| {
            if let Err(e) = sender_.send(AppMessage::HotKey(e)) {
                tracing::error!("Failed to send `AppMessage::HotKey`: {e}");
            }
            event_loop_proxy_.wake_up();
        }));

        let menu = Menu::with_items(&[
            &MenuItem::with_id("show", "Show Launcher", true, None),
            &MenuItem::with_id("settings", "Settings (soon)", false, None),
            &MenuItem::with_id("quit", "Quit", true, None),
        ])?;

        let mut tray_icon = TrayIconBuilder::new()
            .with_menu(Box::new(menu))
            .with_tooltip("kal");

        #[cfg(windows)]
        {
            let icon = tray_icon::Icon::from_resource(2, Some((32, 32)))?;
            tray_icon = tray_icon.with_icon(icon);
        }

        let tray_icon = tray_icon.build()?;

        let event_loop_proxy_ = event_loop_proxy.clone();
        let sender_ = sender.clone();
        TrayIconEvent::set_event_handler(Some(move |e| {
            if let Err(e) = sender_.send(AppMessage::TrayIcon(e)) {
                tracing::error!("Failed to send `AppMessage::TrayIcon`: {e}");
            }
            event_loop_proxy_.wake_up();
        }));

        let event_loop_proxy_ = event_loop_proxy.clone();
        let sender_ = sender.clone();
        MenuEvent::set_event_handler(Some(move |e| {
            if let Err(e) = sender_.send(AppMessage::Menu(e)) {
                tracing::error!("Failed to send `AppMessage::Menu`: {e}");
            }
            event_loop_proxy_.wake_up();
        }));

        let icon_service = Arc::new(icon::Service::new(&kal_data_dir));

        #[cfg(debug_assertions)]
        let web_context = WebContext::new(None);
        #[cfg(not(debug_assertions))]
        let web_context = {
            let data_directory = kal_data_dir.join("kal.exe.WebView2");
            WebContext::new(Some(data_directory))
        };

        Ok(Self {
            event_loop_proxy,
            sender,
            receiver,
            config,
            global_hotkey_manager,
            windows: HashMap::default(),
            #[cfg(windows)]
            previously_foreground_hwnd: HWND::default(),
            icon_service,
            tray_icon,
            web_context,
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

    fn main_window(&self) -> &WebViewWindow {
        self.windows.get(MainWindowState::ID).unwrap()
    }

    pub fn show_main_window(&mut self) -> anyhow::Result<()> {
        #[cfg(windows)]
        self.store_foreground_hwnd();

        let main_window = self.main_window();
        main_window.window().set_visible(true);
        main_window.window().focus_window();
        main_window.emit(IpcEvent::FocusInput, ())
    }

    pub fn hide_main_window(&self, #[allow(unused)] restore_focus: bool) {
        self.main_window().window().set_visible(false);

        #[cfg(windows)]
        if restore_focus {
            self.restore_prev_foreground_hwnd();
        }
    }

    #[cfg(windows)]
    fn listen_for_settings_change(&self, event_loop: &dyn ActiveEventLoop) {
        use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
        use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
        use winit::platform::windows::ActiveEventLoopExtWindows;

        tracing::debug!("Listening for system settings change...");

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
                let userdata = userdata as *const (mpsc::Sender<AppMessage>, EventLoopProxy);
                let (sender, proxy) = &*userdata;

                match sender.send(AppMessage::SystemSettingsChanged) {
                    Ok(_) => proxy.wake_up(),
                    Err(e) => {
                        tracing::error!("Failed to send `AppMessage::SystemSettingsChanged`: {e}")
                    }
                }
            }

            DefSubclassProc(hwnd, umsg, wparam, lparam)
        }
    }

    fn app_message(
        &mut self,
        event_loop: &dyn ActiveEventLoop,
        message: AppMessage,
    ) -> anyhow::Result<()> {
        let span = tracing::debug_span!("app::handle::message", ?message);
        let _enter = span.enter();

        match message {
            #[cfg(all(not(debug_assertions), windows))]
            AppMessage::WebviewFocused(window_id, focus) => {
                let main_window = self.main_window().window();
                // hide main window when it loses focus
                if window_id == main_window.id() && !focus {
                    main_window.set_visible(false);
                }
            }

            AppMessage::HotKey(e) => {
                if e.state == HotKeyState::Pressed {
                    if self.main_window().window().is_visible().unwrap_or_default() {
                        self.hide_main_window(true);
                    } else {
                        self.show_main_window()?;
                    }
                }
            }

            AppMessage::TrayIcon(e) => {
                if let TrayIconEvent::DoubleClick {
                    button: tray_icon::MouseButton::Left,
                    ..
                } = e
                {
                    self.show_main_window()?;
                }
            }

            AppMessage::Menu(e) => match e.id.as_ref() {
                "show" => self.show_main_window()?,
                "quit" => event_loop.exit(),
                _ => {}
            },

            #[cfg(windows)]
            AppMessage::SystemSettingsChanged => {
                if let Ok(colors) = kal_utils::SystemAccentColors::load() {
                    for window in self.windows.values() {
                        window.emit(IpcEvent::UpdateSystemAccentColor, colors)?;
                    }
                }
            }

            AppMessage::RequestSufaceSize(size) => {
                let _ = self.main_window().window().request_surface_size(size);
            }

            AppMessage::HideMainWindow(restore_focus) => self.hide_main_window(restore_focus),

            AppMessage::MainWindowEmit(event, payload) => {
                self.main_window().emit(event, payload)?
            }

            AppMessage::ReRegisterHotKey(old_hotkey, new_hotkey) => {
                self.global_hotkey_manager.unregister(old_hotkey)?;
                self.global_hotkey_manager.register(new_hotkey)?;
            }
        }

        Ok(())
    }
}

impl ApplicationHandler for App {
    fn new_events(&mut self, event_loop: &dyn ActiveEventLoop, cause: winit::event::StartCause) {
        if cause == winit::event::StartCause::Init {
            tracing::debug!("Eventloop initialized");

            #[cfg(windows)]
            self.listen_for_settings_change(event_loop);
        }
    }

    fn exiting(&mut self, _event_loop: &dyn ActiveEventLoop) {
        tracing::debug!("Eventloop Exited");
    }

    fn can_create_surfaces(&mut self, event_loop: &dyn ActiveEventLoop) {
        self.create_main_window(event_loop)
            .expect("Failed to create main window");
    }

    fn proxy_wake_up(&mut self, event_loop: &dyn ActiveEventLoop) {
        tracing::trace!("Eventloop awakaned by proxy");

        while let Ok(message) = self.receiver.try_recv() {
            if let Err(e) = self.app_message(event_loop, message) {
                tracing::error!("Error while handling app message: {e}");
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

            #[cfg(all(not(debug_assertions), not(windows)))]
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
