use std::{borrow::Cow, path::PathBuf, str::FromStr};

use crate::icon;

use wry::{
    http::{header::CONTENT_TYPE, Request, Response},
    WebViewId,
};

/// `kal://` protocol
#[cfg(not(debug_assertions))]
fn kal_inner<'a>(request: Request<Vec<u8>>) -> Result<Response<Cow<'a, [u8]>>, anyhow::Error> {
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

/// `kal://` protocol
#[cfg(not(debug_assertions))]
#[tracing::instrument]
pub fn kal<'a>(_: WebViewId, request: Request<Vec<u8>>) -> Response<Cow<'a, [u8]>> {
    match kal_inner(request) {
        Ok(res) => res,
        Err(e) => Response::builder()
            .status(500)
            .body(e.to_string().as_bytes().to_vec())
            .unwrap()
            .map(Into::into),
    }
}

/// `kalasset://` protocol
fn kal_asset_inner<'a>(
    request: Request<Vec<u8>>,
) -> Result<Response<Cow<'a, [u8]>>, anyhow::Error> {
    let path = &request.uri().path()[1..];
    let path = percent_encoding::percent_decode_str(path).decode_utf8()?;

    let query = request.uri().query();
    if query.map(|q| q.contains("type=builtin")).unwrap_or(false) {
        return Response::builder()
            .header(CONTENT_TYPE, "image/png")
            .body(Cow::Borrowed(
                icon::BuiltinIcon::from_str(path.as_ref())?.bytes(),
            ))
            .map_err(Into::into);
    }

    let path = dunce::canonicalize(PathBuf::from(&*path))?;

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
}

/// `kalasset://` protocol
#[tracing::instrument]
pub fn kal_asset<'a>(_: WebViewId, request: Request<Vec<u8>>) -> Response<Cow<'a, [u8]>> {
    match kal_asset_inner(request) {
        Ok(res) => res,
        Err(e) => Response::builder()
            .status(500)
            .body(e.to_string().as_bytes().to_vec())
            .unwrap()
            .map(Into::into),
    }
}
