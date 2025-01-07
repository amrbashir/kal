use std::{fmt::Debug, rc::Rc};

use crate::{
    event::{AppEvent, WebviewEvent},
    protocol,
};

#[cfg(windows)]
use tao::platform::windows::*;
use tao::{
    dpi::{Position, Size},
    event_loop::EventLoop,
    window::{Window, WindowId},
};
use wry::WebView;

pub struct WebViewWindowBuilder<'a> {
    window_builder: tao::window::WindowBuilder,
    webview_builder: wry::WebViewBuilder<'a>,
}

impl<'a> WebViewWindowBuilder<'a> {
    pub fn new() -> Self {
        Self {
            window_builder: tao::window::WindowBuilder::new(),
            webview_builder: wry::WebViewBuilder::new(),
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

    pub fn transparent(mut self, transparent: bool) -> Self {
        self.window_builder = self.window_builder.with_transparent(transparent);
        self.webview_builder = self.webview_builder.with_transparent(transparent);
        self
    }

    pub fn skip_taskbar(mut self, skip_taskbar: bool) -> Self {
        self.window_builder = self.window_builder.with_skip_taskbar(skip_taskbar);
        self
    }

    pub fn url(mut self, url: &str) -> Self {
        self.webview_builder = self.webview_builder.with_url(url);
        self
    }

    pub fn initialization_script(mut self, script: &str) -> Self {
        self.webview_builder = self.webview_builder.with_initialization_script(script);
        self
    }

    pub fn initialization_script_opt(mut self, script: Option<&str>) -> Self {
        if let Some(script) = script {
            self.webview_builder = self.webview_builder.with_initialization_script(script);
        }
        self
    }

    pub fn devtools(mut self, devtools: bool) -> Self {
        self.webview_builder = self.webview_builder.with_devtools(devtools);
        self
    }

    #[cfg(windows)]
    #[inline]
    fn create_window_surface(window: Rc<Window>) -> SoftBufferContext {
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window).unwrap();
        SoftBufferContext { context, surface }
    }

    unsafe fn attach_webview_focus_handler(
        webview: &WebView,
        event_loop: &EventLoop<AppEvent>,
        window_id: WindowId,
    ) -> anyhow::Result<()> {
        use wry::WebViewExtWindows;
        let mut token = std::mem::zeroed();
        let controller = webview.controller();
        let proxy = event_loop.create_proxy();
        controller.add_GotFocus(
            &webview2_com::FocusChangedEventHandler::create(Box::new(move |_, _| {
                if let Err(e) = proxy.send_event(AppEvent::WebviewEvent {
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
                if let Err(e) = proxy.send_event(AppEvent::WebviewEvent {
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

    pub fn build(mut self, event_loop: &'a EventLoop<AppEvent>) -> anyhow::Result<WebViewWindow> {
        #[cfg(windows)]
        let is_transparent = self.window_builder.window.transparent;

        let window = self
            .window_builder
            .with_window_classname("KalWindowClass")
            .build(event_loop)?;
        let window = Rc::new(window);
        let window_id = window.id();

        #[cfg(windows)]
        let softbuffer_ctx = if is_transparent {
            Some(Self::create_window_surface(window.clone()))
        } else {
            None
        };

        #[cfg(not(debug_assertions))]
        {
            self.webview_builder = self
                .webview_builder
                .with_custom_protocol("kal".into(), protocol::kal);
        }
        self.webview_builder = self
            .webview_builder
            .with_custom_protocol("kalasset".into(), protocol::kal_asset);

        let proxy = event_loop.create_proxy();
        self.webview_builder = self.webview_builder.with_ipc_handler(move |r| {
            if let Err(e) = proxy.send_event(AppEvent::Ipc(window_id, r.body().clone())) {
                tracing::error!("{e}");
            }
        });

        let webview = self.webview_builder.build(&window)?;

        #[cfg(windows)]
        unsafe { Self::attach_webview_focus_handler(&webview, event_loop, window_id) }?;

        let mut webview_window = WebViewWindow {
            window,
            webview,
            #[cfg(windows)]
            softbuffer_ctx,
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
    softbuffer_ctx: Option<SoftBufferContext>,
}

impl Debug for WebViewWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebviewWindow")
            .field("id", &self.window.id())
            .finish()
    }
}

impl tao::rwh_06::HasWindowHandle for WebViewWindow {
    fn window_handle(&self) -> Result<tao::rwh_06::WindowHandle<'_>, tao::rwh_06::HandleError> {
        self.window.window_handle()
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

    #[cfg(windows)]
    pub fn clear_window_surface(&mut self) -> anyhow::Result<()> {
        use std::num::NonZeroU32;

        let Some(softbuffer_ctx) = self.softbuffer_ctx.as_mut() else {
            return Ok(());
        };

        let size = self.window.inner_size();

        let Some(width) = NonZeroU32::new(size.width) else {
            return Ok(());
        };

        let Some(height) = NonZeroU32::new(size.height) else {
            return Ok(());
        };

        softbuffer_ctx
            .surface
            .resize(width, height)
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        let mut buffer = softbuffer_ctx
            .surface
            .buffer_mut()
            .map_err(|e| anyhow::anyhow!("{e}"))?;

        buffer.fill(0);

        buffer.present().map_err(|e| anyhow::anyhow!("{e}"))?;

        Ok(())
    }
}
