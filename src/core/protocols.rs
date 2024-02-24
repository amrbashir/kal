use std::{borrow::Cow, path::PathBuf};

use crate::{common::icon, KAL_DATA_DIR};

use wry::http::{header::CONTENT_TYPE, Request, Response};

const EMPTY_BODY: [u8; 0] = [0_u8; 0];

#[inline]
#[cfg(not(debug_assertions))]
/// `kal://` protocol
#[tracing::instrument]
pub fn kal<'a>(request: Request<Vec<u8>>) -> Result<Response<Cow<'a, [u8]>>, wry::Error> {
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
#[tracing::instrument]
pub fn kal_asset<'a>(request: Request<Vec<u8>>) -> Result<Response<Cow<'a, [u8]>>, wry::Error> {
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

pub(crate) use bail500;
