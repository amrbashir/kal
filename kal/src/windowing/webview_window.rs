use std::borrow::Cow;
use std::cell::RefCell;
use std::fmt::Debug;
use std::rc::Rc;

use anyhow::Context;
use serde::Serialize;
use tao::dpi::{LogicalPosition, Position, Size};
use tao::event_loop::EventLoop;
#[cfg(windows)]
use tao::platform::windows::*;
use tao::window::{Window, WindowBuilder};
use wry::http::{Method, Request, Response};
use wry::{WebView, WebViewBuilder, WebViewId};

use super::ipc;
use super::vibrancy::Vibrancy;
use crate::{icon, AppEvent, AppState};

type IpcHandler = dyn Fn(
    &Rc<RefCell<AppState<AppEvent>>>,
    Request<Vec<u8>>,
) -> anyhow::Result<Response<Cow<'static, [u8]>>>;

pub struct WebViewWindowBuilder<'a> {
    window_builder: tao::window::WindowBuilder,
    webview_builder: wry::WebViewBuilder<'a>,
    center: bool,
    vibrancy: Option<Vibrancy>,
    ipc_handler: Option<&'static IpcHandler>,
}

impl<'a> WebViewWindowBuilder<'a> {
    pub fn new() -> Self {
        Self {
            window_builder: WindowBuilder::new().with_window_classname("KalWindowClass"),
            webview_builder: WebViewBuilder::new()
                .with_initialization_script(include_str!("./ipc.js")),
            center: false,
            vibrancy: None,
            ipc_handler: None,
        }
    }

    pub fn inner_size<S: Into<Size>>(mut self, size: S) -> Self {
        self.window_builder = self.window_builder.with_inner_size(size);
        self
    }

    pub fn position<P: Into<Position>>(mut self, position: P) -> Self {
        self.window_builder = self.window_builder.with_position(position);
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.window_builder = self.window_builder.with_decorations(decorations);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.window_builder = self.window_builder.with_resizable(resizable);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.window_builder = self.window_builder.with_visible(visible);
        self
    }

    pub fn center(mut self, center: bool) -> Self {
        self.center = center;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.window_builder = self.window_builder.with_transparent(transparent);
        self.webview_builder = self.webview_builder.with_transparent(transparent);
        self
    }

    pub fn skip_taskbar(mut self, skip_taskbar: bool) -> Self {
        self.window_builder = self.window_builder.with_skip_taskbar(skip_taskbar);
        self
    }

    pub fn vibrancy(mut self, vibrancy: Option<Vibrancy>) -> Self {
        self.vibrancy = vibrancy;
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.webview_builder = self.webview_builder.with_url(url);
        self
    }

    pub fn init_script(mut self, script: &str) -> Self {
        self.webview_builder = self.webview_builder.with_initialization_script(script);
        self
    }

    pub fn init_script_opt(mut self, script: Option<&str>) -> Self {
        if let Some(script) = script {
            self.webview_builder = self.webview_builder.with_initialization_script(script);
        }
        self
    }

    pub fn ipc(mut self, handler: &'static IpcHandler) -> Self {
        self.ipc_handler.replace(handler);
        self
    }

    pub fn protocol<F>(mut self, name: &str, handler: F) -> Self
    where
        F: Fn(WebViewId, Request<Vec<u8>>) -> anyhow::Result<Response<Cow<'static, [u8]>>>
            + 'static,
    {
        self.webview_builder =
            self.webview_builder
                .with_custom_protocol(name.to_string(), move |webview_id, req| {
                    match handler(webview_id, req) {
                        Ok(res) => res,
                        Err(e) => ipc::error_response_owned(e.to_string()).unwrap(),
                    }
                });
        self
    }

    pub fn devtools(mut self, devtools: bool) -> Self {
        self.webview_builder = self.webview_builder.with_devtools(devtools);
        self
    }

    #[cfg(all(windows, not(debug_assertions)))]
    unsafe fn attach_webview_focus_handler(
        webview: &WebView,
        event_loop: &EventLoop<AppEvent>,
        window_id: tao::window::WindowId,
    ) -> anyhow::Result<()> {
        use wry::WebViewExtWindows;

        use crate::WebviewEvent;

        let mut token = std::mem::zeroed();
        let controller = webview.controller();
        let proxy = event_loop.create_proxy();
        controller.add_GotFocus(
            &webview2_com::FocusChangedEventHandler::create(Box::new(move |_, _| {
                if let Err(e) = proxy.send_event(AppEvent::Webview {
                    event: WebviewEvent::Focus(true),
                    window_id,
                }) {
                    tracing::error!("{e}");
                }
                Ok(())
            })),
            &mut token,
        )?;

        let proxy = event_loop.create_proxy();
        controller.add_LostFocus(
            &webview2_com::FocusChangedEventHandler::create(Box::new(move |_, _| {
                if let Err(e) = proxy.send_event(AppEvent::Webview {
                    event: WebviewEvent::Focus(false),
                    window_id,
                }) {
                    tracing::error!("{e}");
                }
                Ok(())
            })),
            &mut token,
        )?;

        Ok(())
    }

    pub fn build(
        mut self,
        event_loop: &'a EventLoop<AppEvent>,
        app_state: &'a Rc<RefCell<AppState<AppEvent>>>,
    ) -> anyhow::Result<WebViewWindow> {
        self = self.protocol("kalicon", icon::kalicon_protocol);
        #[cfg(not(debug_assertions))]
        {
            self = self.protocol(
                super::embedded_assets::PROTOCOL_NAME,
                super::embedded_assets::protocol,
            );
        }

        if let Some(ipc_handler) = self.ipc_handler.take() {
            let app_state_c = app_state.clone();
            self = self.protocol("kalipc", move |_, request| match *request.method() {
                Method::OPTIONS => ipc::empty_response(),
                Method::POST => ipc_handler(&app_state_c, request),
                _ => ipc::error_response("Only POST or OPTIONS method are supported"),
            });
        }

        if self.center {
            let primary_monitor = event_loop
                .primary_monitor()
                .with_context(|| "Failed to get primary monitor")?;
            let m_size = primary_monitor
                .size()
                .to_logical::<u32>(primary_monitor.scale_factor());
            let m_pos = primary_monitor
                .position()
                .to_logical::<u32>(primary_monitor.scale_factor());

            let width = self
                .window_builder
                .window
                .inner_size
                .map(|s| s.to_logical(primary_monitor.scale_factor()).width)
                .unwrap_or(800);

            self = self.position(LogicalPosition::new(
                m_pos.x + (m_size.width / 2 - width / 2),
                m_pos.y + (m_size.height / 4),
            ));
        }

        let window = self.window_builder.build(event_loop)?;

        if let Some(vibrancy) = self.vibrancy {
            vibrancy.apply(&window)?;
        }

        let window = Rc::new(window);

        let webview = self.webview_builder.build(&window)?;

        #[cfg(all(windows, not(debug_assertions)))]
        unsafe {
            let window_id = window.id();
            Self::attach_webview_focus_handler(&webview, event_loop, window_id)
        }?;

        let mut webview_window = WebViewWindow {
            window: window.clone(),
            webview,
            #[cfg(windows)]
            softbuffer_ctx: {
                let context = softbuffer::Context::new(window.clone()).unwrap();
                let surface = softbuffer::Surface::new(&context, window).unwrap();
                SoftBufferContext { context, surface }
            },
        };

        #[cfg(windows)]
        webview_window.clear_window_surface()?;

        Ok(webview_window)
    }
}

#[cfg(windows)]
struct SoftBufferContext {
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    #[allow(unused)]
    context: softbuffer::Context<Rc<Window>>,
}

pub struct WebViewWindow {
    window: Rc<Window>,
    webview: WebView,
    #[cfg(windows)]
    softbuffer_ctx: SoftBufferContext,
}

impl Debug for WebViewWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebviewWindow")
            .field("id", &self.window.id())
            .finish()
    }
}

impl WebViewWindow {
    #[inline(always)]
    pub fn window(&self) -> &Window {
        self.window.as_ref()
    }

    #[inline(always)]
    pub fn webview(&self) -> &WebView {
        &self.webview
    }

    pub fn emit(&self, event: impl AsRef<str>, payload: impl Serialize) -> anyhow::Result<()> {
        ipc::emit(self.webview(), event, payload)
    }

    #[cfg(windows)]
    pub fn clear_window_surface(&mut self) -> anyhow::Result<()> {
        use std::num::NonZeroU32;

        let size = self.window.inner_size();

        let Some(width) = NonZeroU32::new(size.width) else {
            return Ok(());
        };

        let Some(height) = NonZeroU32::new(size.height) else {
            return Ok(());
        };

        self.softbuffer_ctx
            .surface
            .resize(width, height)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut buffer = self
            .softbuffer_ctx
            .surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        buffer.fill(0);

        buffer.present().map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(())
    }
}
