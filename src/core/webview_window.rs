use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    rc::Rc,
};

use crate::{
    event::{AppEvent, WebviewEvent},
    protocols,
};

use tao::{
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};
use wry::{http::Response, WebView, WebViewAttributes, WebViewBuilder};

pub struct WebviewWindow {
    pub window: Rc<Window>,
    pub webview: WebView,

    #[cfg(windows)]
    pub is_transparent: bool,
    #[cfg(windows)]
    pub sb_surface: softbuffer::Surface<Rc<Window>, Rc<Window>>,
    #[cfg(windows)]
    pub sb_ctx: softbuffer::Context<Rc<Window>>,
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

impl Deref for WebviewWindow {
    type Target = WebView;

    fn deref(&self) -> &Self::Target {
        &self.webview
    }
}

impl DerefMut for WebviewWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.webview
    }
}

impl WebviewWindow {
    pub fn new(
        window_options: WindowAttributes,
        webview_options: WebViewAttributes,
        event_loop: &EventLoop<AppEvent>,
    ) -> anyhow::Result<Self> {
        #[cfg(windows)]
        let is_transparent = window_options.transparent;

        let mut builder = tao::window::WindowBuilder::new();
        builder.window = window_options;
        let window = builder.build(event_loop)?;

        #[cfg(windows)]
        let (window, context, surface) = {
            let window = Rc::new(window);
            let context = softbuffer::Context::new(window.clone()).unwrap();
            let surface = softbuffer::Surface::new(&context, window.clone()).unwrap();
            (window, context, surface)
        };

        let mut builder = WebViewBuilder::new(&window);
        builder.attrs = webview_options;

        #[cfg(not(debug_assertions))]
        {
            builder = builder.with_custom_protocol("kal".into(), move |request| {
                protocols::bail500!(protocols::kal(request))
            });
        }
        builder = builder.with_custom_protocol("kalasset".into(), move |request| {
            protocols::bail500!(protocols::kal_asset(request))
        });

        let proxy = event_loop.create_proxy();
        let window_id = window.id();
        builder = builder.with_ipc_handler(move |r| {
            if let Err(e) = proxy.send_event(AppEvent::Ipc(window_id, r)) {
                tracing::error!("{e}");
            }
        });

        let webview = builder.build()?;

        #[cfg(windows)]
        {
            use wry::WebViewExtWindows;
            let mut token = unsafe { std::mem::zeroed() };
            let controller = webview.controller();
            unsafe {
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
            }
        }

        let mut webview_window = WebviewWindow {
            window,
            webview,

            #[cfg(windows)]
            is_transparent,
            #[cfg(windows)]
            sb_surface: surface,
            #[cfg(windows)]
            sb_ctx: context,
        };

        #[cfg(windows)]
        webview_window.clear_window_surface();

        Ok(webview_window)
    }

    #[cfg(windows)]
    pub fn clear_window_surface(&mut self) {
        let size = self.window.inner_size();
        if let (Some(width), Some(height)) = (
            std::num::NonZeroU32::new(size.width),
            std::num::NonZeroU32::new(size.height),
        ) {
            self.sb_surface.resize(width, height).unwrap();
            let mut buffer = self.sb_surface.buffer_mut().unwrap();
            buffer.fill(0);
            let _ = buffer.present();
        }
    }
}
