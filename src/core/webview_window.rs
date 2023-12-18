use std::{
    borrow::Cow,
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use crate::{
    common::icon,
    event::{AppEvent, WebviewEvent},
    KAL_DATA_DIR,
};

use tao::{
    event_loop::EventLoop,
    platform::windows::WindowBuilderExtWindows,
    window::{Window, WindowAttributes},
};
use wry::{
    http::{header::CONTENT_TYPE, Request, Response},
    WebView, WebViewAttributes, WebViewBuilder,
};

pub struct WebviewWindow {
    pub window: Window,
    pub webview: WebView,
}

impl Debug for WebviewWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebviewWindow")
            .field("id", &self.window.id())
            .finish()
    }
}

impl WebviewWindow {
    pub fn new(
        window_options: WindowAttributes,
        webview_options: WebViewAttributes,
        event_loop: &EventLoop<AppEvent>,
    ) -> anyhow::Result<Self> {
        let mut builder = tao::window::WindowBuilder::new();
        builder.window = window_options;
        #[cfg(windows)]
        {
            let enable = builder.window.decorations;
            builder = builder.with_undecorated_shadow(enable);
        }
        let window = builder.build(event_loop)?;

        let mut builder = WebViewBuilder::new(&window);
        builder.attrs = webview_options;

        #[cfg(not(debug_assertions))]
        {
            builder =
                builder.with_custom_protocol("kal".into(), move |request| {
                    match kal_protocol(request) {
                        Ok(r) => r,
                        Err(e) => Response::builder()
                            .status(500)
                            .body(e.to_string().as_bytes().to_vec())
                            .unwrap()
                            .map(Into::into),
                    }
                });
        }
        builder =
            builder.with_custom_protocol(
                "kalasset".into(),
                move |request| match kal_asset_protocol(request) {
                    Ok(r) => r,
                    Err(e) => Response::builder()
                        .status(500)
                        .body(e.to_string().as_bytes().to_vec())
                        .unwrap()
                        .map(Into::into),
                },
            );

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
        Ok(WebviewWindow { window, webview })
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

#[cfg(not(debug_assertions))]
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
