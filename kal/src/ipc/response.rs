use std::borrow::Cow;

use wry::http::header::*;
use wry::http::{response, Response};

#[inline]
pub fn base() -> response::Builder {
    Response::builder()
        .header(ACCESS_CONTROL_ALLOW_ORIGIN, "*")
        .header(ACCESS_CONTROL_ALLOW_HEADERS, "*")
        .header(ACCESS_CONTROL_ALLOW_METHODS, "*")
}

#[inline]
pub fn empty<'a>() -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    base().status(200).body((&[]).into()).map_err(Into::into)
}

#[inline]
pub fn error<S: AsRef<str> + ?Sized>(error: &S) -> anyhow::Result<Response<Cow<'_, [u8]>>> {
    base()
        .header(CONTENT_TYPE, "text/plain")
        .status(500)
        .body(error.as_ref().as_bytes().into())
        .map_err(Into::into)
}

#[inline]
pub fn error_owned<'a, S: AsRef<str>>(error: S) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    base()
        .header(CONTENT_TYPE, "text/plain")
        .status(500)
        .body(error.as_ref().as_bytes().to_vec().into())
        .map_err(Into::into)
}

#[inline]
pub fn json<'a>(json: &impl serde::Serialize) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    base()
        .header(CONTENT_TYPE, "application/json")
        .status(200)
        .body(serde_json::to_vec(json)?.into())
        .map_err(Into::into)
}
