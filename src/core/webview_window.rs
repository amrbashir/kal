use std::{fmt::Debug, rc::Rc};

use crate::{
    event::{AppEvent, WebviewEvent},
    protocol,
};

#[cfg(windows)]
use tao::platform::windows::*;
use tao::{
    event_loop::EventLoop,
    window::{Window, WindowAttributes, WindowId},
};
use wry::{WebView, WebViewAttributes, WebViewBuilder};

#[cfg(windows)]
struct SoftBufferContext {
    surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    #[allow(unused)]
    context: softbuffer::Context<Rc<Window>>,
}

pub struct WebviewWindow {
    window: Rc<Window>,
    webview: WebView,
    #[cfg(windows)]
    softbuffer_ctx: Option<SoftBufferContext>,
}

impl Debug for WebviewWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebviewWindow")
            .field("id", &self.window.id())
            .finish()
    }
}

impl tao::rwh_06::HasWindowHandle for WebviewWindow {
    fn window_handle(&self) -> Result<tao::rwh_06::WindowHandle<'_>, tao::rwh_06::HandleError> {
        self.window.window_handle()
    }
}

impl WebviewWindow {
    pub fn new(
        window_attrs: WindowAttributes,
        webview_atts: WebViewAttributes,
        event_loop: &EventLoop<AppEvent>,
    ) -> anyhow::Result<Self> {
        #[cfg(windows)]
        let is_transparent = window_attrs.transparent;

        let mut builder = tao::window::WindowBuilder::new();
        builder.window = window_attrs;

        #[cfg(windows)]
        {
            builder = builder.with_window_classname("KalWindowClass");
        }

        let window = builder.build(event_loop)?;

        let window = Rc::new(window);

        let window_id = window.id();

        #[cfg(windows)]
        let softbuffer_ctx = if is_transparent {
            Some(Self::create_window_surface(window.clone()))
        } else {
            None
        };

        let mut builder = WebViewBuilder::new(&window);
        builder.attrs = webview_atts;

        builder = Self::attach_protocols(builder);
        builder = Self::attach_ipc_handler(builder, event_loop, window_id);

        let webview = builder.build()?;

        #[cfg(windows)]
        unsafe { Self::attach_webview_focus_handler(&webview, event_loop, window_id) }?;

        let mut webview_window = WebviewWindow {
            window,
            webview,
            #[cfg(windows)]
            softbuffer_ctx,
        };

        #[cfg(windows)]
        webview_window.clear_window_surface()?;

        Ok(webview_window)
    }

    fn attach_protocols(mut builder: WebViewBuilder) -> WebViewBuilder {
        #[cfg(not(debug_assertions))]
        {
            builder = builder.with_custom_protocol("kal".into(), protocol::kal);
        }

        builder = builder.with_custom_protocol("kalasset".into(), protocol::kal_asset);

        builder
    }

    fn attach_ipc_handler<'a>(
        mut builder: WebViewBuilder<'a>,
        event_loop: &'a EventLoop<AppEvent>,
        window_id: WindowId,
    ) -> WebViewBuilder<'a> {
        let proxy = event_loop.create_proxy();
        builder = builder.with_ipc_handler(move |r| {
            if let Err(e) = proxy.send_event(AppEvent::Ipc(window_id, r.body().clone())) {
                tracing::error!("{e}");
            }
        });
        builder
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

    #[cfg(windows)]
    #[inline]
    fn create_window_surface(window: Rc<Window>) -> SoftBufferContext {
        let context = softbuffer::Context::new(window.clone()).unwrap();
        let surface = softbuffer::Surface::new(&context, window).unwrap();
        SoftBufferContext { context, surface }
    }
}

impl WebviewWindow {
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
