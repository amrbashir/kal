use std::{
    fmt::Debug,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

use crate::event::{AppEvent, WebviewEvent};

use wry::{
    application::{event_loop::EventLoop, window::WindowAttributes},
    http::ResponseBuilder,
    webview::{WebView, WebViewAttributes, WebViewBuilder},
};

pub struct WebviewWindow(WebView);

impl Debug for WebviewWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("WebviewWindow")
            .field("id", &self.0.window().id())
            .finish()
    }
}

impl WebviewWindow {
    pub fn new(
        window_options: WindowAttributes,
        webview_options: WebViewAttributes,
        event_loop: &EventLoop<AppEvent>,
    ) -> anyhow::Result<Self> {
        let mut builder = wry::application::window::WindowBuilder::new();
        builder.window = window_options;
        let window = builder.build(&event_loop)?;

        let mut builder = WebViewBuilder::new(window)?;
        builder.webview = webview_options;
        #[cfg(not(debug_assertions))]
        builder.webview.custom_protocols.push((
            "kal".into(),
            Box::new(move |request| {
                let path = request.uri().replace("kal://localhost/", "");
                let data = crate::EmbededAssets::get(&path)
                    .unwrap_or_else(|| crate::EmbededAssets::get("index.html").unwrap())
                    .data;
                let mimetype = match &*PathBuf::from(path)
                    .extension()
                    .unwrap_or_default()
                    .to_string_lossy()
                {
                    "html" | "htm" => "text/html",
                    "js" | "mjs" => "text/javascript",
                    "css" => "text/css",
                    "png" => "image/png",
                    "jpg" | "jpeg" => "image/jpeg",
                    "svg" => "image/svg+xml",
                    _ => "text/html",
                };

                ResponseBuilder::new()
                    .mimetype(mimetype)
                    .body(data.to_vec())
            }),
        ));
        builder.webview.custom_protocols.push((
            "kalasset".into(),
            Box::new(move |request| {
                let path = request.uri().replace("kalasset://localhost/", "");
                let path = percent_encoding::percent_decode_str(&path).decode_utf8_lossy();
                let path = dunce::canonicalize(PathBuf::from(path.to_string())).unwrap_or_default();

                let assets_dir = dirs_next::data_local_dir()
                    .expect("Failed to get $data_local_dir path")
                    .join("kal");

                if path.starts_with(assets_dir) {
                    let mimetype = match &*path.extension().unwrap_or_default().to_string_lossy() {
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "svg" => "image/svg+xml",
                        _ => "text/html",
                    };

                    ResponseBuilder::new()
                        .mimetype(mimetype)
                        .body(std::fs::read(path).unwrap_or_default())
                } else {
                    ResponseBuilder::new().status(403).body([].into())
                }
            }),
        ));

        let webview = builder.build()?;

        #[cfg(target_os = "windows")]
        {
            use wry::webview::WebviewExtWindows;
            let mut token = unsafe { std::mem::zeroed() };
            let controller = webview.controller();
            let window_id = webview.window().id();
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
        Ok(WebviewWindow(webview))
    }
}

impl Deref for WebviewWindow {
    type Target = WebView;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for WebviewWindow {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
