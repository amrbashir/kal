use std::borrow::Cow;
use std::path::PathBuf;

use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};
use wry::WebViewId;

use crate::webview_window::ProtocolResult;

#[derive(rust_embed::RustEmbed)]
#[folder = "../kal-ui/dist"]
pub(crate) struct EmbededAssets;

pub const PROTOCOL_NAME: &str = "kal";

/// `kal://` protocol
pub fn protocol(_webview_id: WebViewId, request: Request<Vec<u8>>) -> ProtocolResult {
    let path = &request.uri().path()[1..];
    let path = percent_encoding::percent_decode_str(path).decode_utf8()?;

    let file =
        EmbededAssets::get(&path).unwrap_or_else(|| EmbededAssets::get("index.html").unwrap());

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
