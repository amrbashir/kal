use std::borrow::Cow;

use serde::Serialize;
use wry::http::header::*;
use wry::http::{response, Response};
use wry::WebView;

pub fn emit(
    webview: &WebView,
    event: impl AsRef<str>,
    payload: impl Serialize,
) -> anyhow::Result<()> {
    let script = format!(
        r#"(function(){{
            window.KAL.ipc.__handler_store['{}'].forEach(handler => {{
                handler({});
            }});
        }})()"#,
        event.as_ref(),
        serialize_to_javascript::Serialized::new(
            &serde_json::value::to_raw_value(&payload).unwrap_or_default(),
            &serialize_to_javascript::Options::default()
        ),
    );

    webview.evaluate_script(&script).map_err(Into::into)
}

#[inline]
pub fn base_response() -> response::Builder {
    Response::builder()
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .header(ACCESS_CONTROL_ALLOW_METHODS, "*")
}

#[inline]
pub fn empty_response<'a>() -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    base_response()
        .status(200)
        .body((&[]).into())
        .map_err(Into::into)
}

#[inline]
pub fn error_response<S: AsRef<str> + ?Sized>(
    error: &S,
) -> anyhow::Result<Response<Cow<'_, [u8]>>> {
    base_response()
        .header(CONTENT_TYPE, "text/plain")
        .status(500)
        .body(error.as_ref().as_bytes().into())
        .map_err(Into::into)
}

#[inline]
pub fn error_response_owned<'a, S: AsRef<str>>(
    error: S,
) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    base_response()
        .header(CONTENT_TYPE, "text/plain")
        .status(500)
        .body(error.as_ref().as_bytes().to_vec().into())
        .map_err(Into::into)
}

#[inline]
pub fn make_json_response<'a>(
    json: &impl serde::Serialize,
) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    base_response()
        .header(CONTENT_TYPE, "application/json")
        .status(200)
        .body(serde_json::to_vec(json)?.into())
        .map_err(Into::into)
}
