use std::borrow::Cow;
use std::fmt::Debug;
use std::future::Future;
use std::rc::Rc;
use std::sync::Arc;

use anyhow::Context;
use serde::{Deserialize, Serialize};
use winit::dpi::{LogicalPosition, Position, Size};
use winit::event_loop::ActiveEventLoop;
#[cfg(windows)]
use winit::platform::windows::*;
use winit::window::{Window, WindowAttributes, WindowId};
use wry::http::{Request, Response};
#[cfg(windows)]
use wry::WebViewBuilderExtWindows;
use wry::{WebView, WebViewBuilder};

use crate::ipc;
use crate::ipc::AsyncIpcMessage;

pub type ProtocolResult = anyhow::Result<Response<Cow<'static, [u8]>>>;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub enum Vibrancy {
    Mica,
    Tabbed,
    Acrylic,
    Blur,
}

pub struct WebViewWindowBuilder<'a> {
    window_attrs: WindowAttributes,
    webview_builder: WebViewBuilder<'a>,
    center: bool,
}

impl WebViewWindowBuilder<'_> {
    pub fn new() -> Self {
        let mut window_attrs = WindowAttributes::default();

        let mut webview_builder = WebViewBuilder::new()
            .with_initialization_script(ipc::INIT_SCRIPT)
            .with_hotkeys_zoom(false);

        #[cfg(windows)]
        {
            window_attrs = window_attrs
                .with_class_name("KalWindowClass")
                .with_undecorated_shadow(true)
                .with_clip_children(false);

            let colors = crate::utils::SystemAccentColors::load().unwrap_or_default();

            webview_builder = webview_builder
                .with_initialization_script(&colors.init_script())
                .with_scroll_bar_style(wry::ScrollBarStyle::FluentOverlay);
        }

        Self {
            window_attrs,
            webview_builder,
            center: false,
        }
    }

    pub fn with_webview_id(mut self, id: &'static str) -> Self {
        self.webview_builder = self.webview_builder.with_id(id);
        self
    }

    pub fn inner_size<S: Into<Size>>(mut self, size: S) -> Self {
        self.window_attrs = self.window_attrs.with_surface_size(size);
        self
    }

    pub fn position<P: Into<Position>>(mut self, position: P) -> Self {
        self.window_attrs = self.window_attrs.with_position(position);
        self
    }

    pub fn decorations(mut self, decorations: bool) -> Self {
        self.window_attrs = self.window_attrs.with_decorations(decorations);
        self
    }

    pub fn resizable(mut self, resizable: bool) -> Self {
        self.window_attrs = self.window_attrs.with_resizable(resizable);
        self
    }

    pub fn visible(mut self, visible: bool) -> Self {
        self.window_attrs = self.window_attrs.with_visible(visible);
        self
    }

    pub fn center(mut self, center: bool) -> Self {
        self.center = center;
        self
    }

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.window_attrs = self.window_attrs.with_transparent(transparent);
        self.webview_builder = self.webview_builder.with_transparent(transparent);
        self
    }

    pub fn skip_taskbar(mut self, skip_taskbar: bool) -> Self {
        #[cfg(windows)]
        {
            self.window_attrs = self.window_attrs.with_skip_taskbar(skip_taskbar);
        }
        self
    }

    pub fn vibrancy(mut self, vibrancy: Option<Vibrancy>) -> Self {
        #[cfg(windows)]
        {
            self.window_attrs = self.window_attrs.with_system_backdrop(match vibrancy {
                Some(Vibrancy::Mica) => BackdropType::MainWindow,
                Some(Vibrancy::Tabbed) => BackdropType::TabbedWindow,
                Some(Vibrancy::Acrylic) => BackdropType::TransientWindow,
                _ => BackdropType::None,
            });
        }
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

    #[cfg(not(debug_assertions))]
    pub fn protocol<F>(mut self, name: &str, handler: F) -> Self
    where
        F: Fn(&str, Request<Vec<u8>>) -> ProtocolResult + 'static,
    {
        self.webview_builder =
            self.webview_builder
                .with_custom_protocol(name.to_string(), move |webview_id, req| {
                    match handler(&webview_id, req) {
                        Ok(res) => res,
                        Err(e) => ipc::response::error_owned(format!("{e:?}")).unwrap(),
                    }
                });
        self
    }

    pub fn async_protocol<F, FR>(mut self, name: &str, handler: F) -> Self
    where
        F: Fn(&str, Request<Vec<u8>>) -> FR + 'static + Send + Sync,
        FR: Future<Output = ProtocolResult> + 'static + Send + Sync,
    {
        let handler = Arc::new(handler);

        self.webview_builder = self.webview_builder.with_asynchronous_custom_protocol(
            name.to_string(),
            move |webview_id, req, responder| {
                let webview_id = webview_id.to_string();
                let handler = handler.clone();

                let fut = async move {
                    let res = match handler(&webview_id, req).await {
                        Ok(res) => res,
                        Err(e) => ipc::response::error_owned(format!("{e:?}")).unwrap(),
                    };

                    responder.respond(res);
                };

                smol::spawn(fut).detach();
            },
        );
        self
    }

    pub fn async_ipc<T>(mut self, async_ipc_sender: smol::channel::Sender<T>) -> Self
    where
        T: From<AsyncIpcMessage> + Send + Sync + 'static,
    {
        self = self.async_protocol(
            ipc::PROTOCOL_NAME,
            ipc::make_async_ipc_protocol!(T, async_ipc_sender),
        );

        self
    }

    pub fn devtools(mut self, devtools: bool) -> Self {
        self.webview_builder = self.webview_builder.with_devtools(devtools);
        self
    }

    pub fn build(mut self, event_loop: &dyn ActiveEventLoop) -> anyhow::Result<WebViewWindow> {
        #[cfg(not(debug_assertions))]
        {
            self = self.protocol(
                crate::embedded_assets::PROTOCOL_NAME,
                crate::embedded_assets::protocol,
            );
        }

        if self.center {
            let primary_monitor = event_loop
                .primary_monitor()
                .with_context(|| "Failed to get primary monitor")?;
            let m_size = primary_monitor
                .current_video_mode()
                .map(|v| v.size())
                .unwrap_or_default()
                .to_logical::<u32>(primary_monitor.scale_factor());
            let m_pos = primary_monitor
                .position()
                .unwrap_or_default()
                .to_logical::<u32>(primary_monitor.scale_factor());

            let width = self
                .window_attrs
                .surface_size
                .map(|s| s.to_logical(primary_monitor.scale_factor()).width)
                .unwrap_or(800);

            self = self.position(LogicalPosition::new(
                m_pos.x + (m_size.width / 2 - width / 2),
                m_pos.y + (m_size.height / 4),
            ));
        }

        let window = event_loop.create_window(self.window_attrs)?;

        let window: Rc<dyn Window> = Rc::from(window);

        let webview = self.webview_builder.build(&window)?;

        let mut webview_window = WebViewWindow {
            window: window.clone(),
            webview,
            #[cfg(windows)]
            softbuffer_ctx: {
                let context =
                    softbuffer::Context::new(window.clone()).map_err(|e| anyhow::anyhow!("{e}"))?;
                let surface = softbuffer::Surface::new(&context, window)
                    .map_err(|e| anyhow::anyhow!("{e}"))?;
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
    surface: softbuffer::Surface<Rc<dyn Window>, Rc<dyn Window>>,
    #[allow(unused)]
    context: softbuffer::Context<Rc<dyn Window>>,
}

pub struct WebViewWindow {
    window: Rc<dyn Window>,
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
    /// Magic number accounting for top and bottom border
    /// for undecorated window with shadows
    pub const MAGIC_BORDERS: u32 = 2;

    #[inline(always)]
    pub fn id(&self) -> WindowId {
        self.window.id()
    }

    #[inline(always)]
    pub fn window(&self) -> &dyn Window {
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
    pub fn set_dwmwa_transitions(&self, enable: bool) {
        use windows::Win32::Foundation::{BOOL, HWND};
        use windows::Win32::Graphics::Dwm::{
            DwmSetWindowAttribute, DWMWA_TRANSITIONS_FORCEDISABLED,
        };
        use wry::raw_window_handle::{HasWindowHandle, RawWindowHandle};

        // disable hiding/showing animations
        let RawWindowHandle::Win32(raw) = self.window.window_handle().unwrap().as_raw() else {
            unreachable!()
        };

        let hwnd = HWND(raw.hwnd.get() as _);
        let disable = BOOL(!enable as _);
        let disable = &disable as *const _ as *const _;
        let flag = DWMWA_TRANSITIONS_FORCEDISABLED;
        let size = std::mem::size_of::<BOOL>() as u32;
        if let Err(e) = unsafe { DwmSetWindowAttribute(hwnd, flag, disable, size) } {
            tracing::error!("Failed to change DWMWA_TRANSITIONS_FORCEDISABLED: {e}");
        }
    }

    #[cfg(windows)]
    pub fn clear_window_surface(&mut self) -> anyhow::Result<()> {
        use std::num::NonZeroU32;

        let size = self.window.surface_size();

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
