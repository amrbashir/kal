use std::ops::{Deref, DerefMut};

use wry::{
    application::{event_loop::EventLoop, window::WindowAttributes},
    http::ResponseBuilder,
    webview::{WebView, WebViewAttributes, WebViewBuilder},
};

use crate::event::{AppEvent, WebviewEvent};

pub struct WebviewWindow(WebView);

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
                let data = crate::EmbededAsset::get(&path)
                    .unwrap_or_else(|| crate::EmbededAsset::get("index.html").unwrap())
                    .data;
                let mimetype = match std::path::PathBuf::from(path)
                    .extension()
                    .unwrap_or_default()
                    .to_str()
                    .unwrap_or_default()
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
                let path = dunce::canonicalize(std::path::PathBuf::from(path.to_string()))
                    .unwrap_or_default();

                let mut assets_dir = dirs_next::home_dir().expect("Failed to get $HOME dir path");
                assets_dir.push(".kal");

                if path.starts_with(assets_dir) {
                    let mimetype = match path
                        .extension()
                        .unwrap_or_default()
                        .to_str()
                        .unwrap_or_default()
                    {
                        "png" => "image/png",
                        "jpg" | "jpeg" => "image/jpeg",
                        "svg" => "image/svg+xml",
                        _ => "text/html",
                    };

                    ResponseBuilder::new()
                        .mimetype(mimetype)
                        .body(std::fs::read(path).unwrap_or_default())
                } else {
                    ResponseBuilder::new().body([].into())
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
                let _ = controller.GotFocus(
                    webview2_com::FocusChangedEventHandler::create(Box::new(move |_, _| {
                        let _ = proxy.send_event(AppEvent::WebviewEvent {
                            event: WebviewEvent::Focus(true),
                            window_id,
                        });
                        Ok(())
                    })),
                    &mut token,
                );
                let proxy = event_loop.create_proxy();
                let _ = controller.LostFocus(
                    webview2_com::FocusChangedEventHandler::create(Box::new(move |_, _| {
                        let _ = proxy.send_event(AppEvent::WebviewEvent {
                            event: WebviewEvent::Focus(false),
                            window_id,
                        });
                        Ok(())
                    })),
                    &mut token,
                );
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
