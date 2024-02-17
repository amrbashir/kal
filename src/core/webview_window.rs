use std::{
    borrow::Cow,
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::PathBuf,
    rc::Rc,
};

use crate::{
    common::icon,
    event::{AppEvent, WebviewEvent},
    KAL_DATA_DIR,
};

use tao::{
    event_loop::EventLoop,
    window::{Window, WindowAttributes},
};
use wry::{
    http::{header::CONTENT_TYPE, Request, Response},
    WebView, WebViewAttributes, WebViewBuilder,
};

macro_rules! bail500 {
    ($res:expr) => {
        match $res {
            Ok(r) => r,
            Err(e) => Response::builder()
                .status(500)
                .body(e.to_string().as_bytes().to_vec())
                .unwrap()
                .map(Into::into),
        }
    };
}

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
            builder = builder
                .with_custom_protocol("kal".into(), move |request| bail500!(kal_protocol(request)));
        }
        builder = builder.with_custom_protocol("kalasset".into(), move |request| {
            bail500!(kal_asset_protocol(request))
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

        #[cfg_attr(not(windows), allow(unused_mut))]
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
        clear_window_surface(&mut webview_window);

        Ok(webview_window)
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

const EMPTY_BODY: [u8; 0] = [0_u8; 0];

#[inline]
#[cfg(not(debug_assertions))]
/// `kal://` protocol
fn kal_protocol<'a>(request: Request<Vec<u8>>) -> Result<Response<Cow<'a, [u8]>>, wry::Error> {
    let path = &request.uri().path()[1..];
    let path = percent_encoding::percent_decode_str(path).decode_utf8()?;

    let file = crate::EmbededAssets::get(&path)
        .unwrap_or_else(|| crate::EmbededAssets::get("index.html").unwrap());

    let path = PathBuf::from(&*path);
    let mimetype = match path.extension().unwrap_or_default().to_str() {
        Some("html") | Some("htm") => "text/html",
        Some("js") | Some("mjs") => "text/javascript",
        Some("css") => "text/css",
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => "text/html",
    };

    Response::builder()
        .header(CONTENT_TYPE, mimetype)
        .body(Cow::from(file.data))
        .map_err(Into::into)
}

#[inline]
/// `kalasset://` protocol
fn kal_asset_protocol<'a>(
    request: Request<Vec<u8>>,
) -> Result<Response<Cow<'a, [u8]>>, wry::Error> {
    let path = &request.uri().path()[1..];
    let path = percent_encoding::percent_decode_str(path).decode_utf8()?;

    if path.starts_with("icons/defaults") {
        return Response::builder()
            .header(CONTENT_TYPE, "image/png")
            .body(Cow::from(icon::Defaults::bytes(&path)))
            .map_err(Into::into);
    }

    let path = dunce::canonicalize(PathBuf::from(&*path))?;

    if path.starts_with(&*KAL_DATA_DIR) {
        let mimetype = match path.extension().unwrap_or_default().to_str() {
            Some("png") => "image/png",
            Some("jpg") | Some("jpeg") => "image/jpeg",
            Some("svg") => "image/svg+xml",
            _ => "text/html",
        };

        Response::builder()
            .header(CONTENT_TYPE, mimetype)
            .body(Cow::from(std::fs::read(path)?))
            .map_err(Into::into)
    } else {
        Response::builder()
            .status(403)
            .body(Cow::from(&EMPTY_BODY[..]))
            .map_err(Into::into)
    }
}

#[cfg(windows)]
pub fn clear_window_surface(window: &mut WebviewWindow) {
    let size = window.window.inner_size();
    if let (Some(width), Some(height)) = (
        std::num::NonZeroU32::new(size.width),
        std::num::NonZeroU32::new(size.height),
    ) {
        window.sb_surface.resize(width, height).unwrap();
        let mut buffer = window.sb_surface.buffer_mut().unwrap();
        buffer.fill(0);
        let _ = buffer.present();
    }
}
