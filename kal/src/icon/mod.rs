use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};
use wry::WebViewId;

use crate::ipc;

mod extract;

pub use self::extract::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconType {
    Path,
    Svg,
    BuiltIn,
    Url,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Icon<'a> {
    pub data: Cow<'a, str>,
    pub r#type: IconType,
}

impl<'a> Icon<'a> {
    #[inline]
    pub fn new(data: Cow<'a, str>, r#type: IconType) -> Self {
        Self { data, r#type }
    }

    #[inline]
    pub fn path(data: Cow<'a, str>) -> Self {
        Self::new(data, IconType::Path)
    }

    #[inline]
    pub fn svg(data: Cow<'a, str>) -> Self {
        Self::new(data, IconType::Svg)
    }

    #[inline]
    pub fn builtin(data: Cow<'a, str>) -> Self {
        Self::new(data, IconType::BuiltIn)
    }
}

#[derive(EnumString, AsRefStr, Clone, Copy)]
pub enum BuiltInIcon {
    Directory,
    Url,
    Shell,
    Shutdown,
    Restart,
    SignOut,
    Hibernate,
    Sleep,
    Lock,
    Calculator,
    Workflow,
}

impl BuiltInIcon {
    pub fn icon(&self) -> Icon<'_> {
        match self {
            Self::Shutdown => Icon::svg(include_str!("./built-in-icons/Shutdown.svg").into()),
            Self::Restart => Icon::svg(include_str!("./built-in-icons/Restart.svg").into()),
            Self::SignOut => Icon::svg(include_str!("./built-in-icons/Signout.svg").into()),
            Self::Hibernate => Icon::svg(include_str!("./built-in-icons/Hibernate.svg").into()),
            Self::Sleep => Icon::svg(include_str!("./built-in-icons/Sleep.svg").into()),
            Self::Directory => Icon::svg(include_str!("./built-in-icons/Folder.svg").into()),
            Self::Lock => Icon::svg(include_str!("./built-in-icons/Lock.svg").into()),
            Self::Calculator => Icon::svg(include_str!("./built-in-icons/Calculator.svg").into()),
            Self::Workflow => Icon::svg(include_str!("./built-in-icons/Workflow.svg").into()),
            _ => Icon::builtin(Cow::Borrowed(self.as_ref())),
        }
    }

    pub fn bytes(&self) -> &'static [u8] {
        match self {
            Self::Url => include_bytes!("./built-in-icons/Url.png"),
            Self::Shell => include_bytes!("./built-in-icons/Shell.png"),
            _ => unreachable!(),
        }
    }
}

pub const PROTOCOL_NAME: &str = "kalicon";

/// `kalicon://` protocol
#[tracing::instrument]
pub fn protocol<'a>(
    _webview_id: WebViewId,
    request: Request<Vec<u8>>,
) -> anyhow::Result<Response<Cow<'a, [u8]>>> {
    let path = &request.uri().path()[1..];
    let path = percent_encoding::percent_decode_str(path).decode_utf8()?;

    let query = request.uri().query();
    if query.map(|q| q.contains("type=builtin")).unwrap_or(false) {
        return Response::builder()
            .header(CONTENT_TYPE, "image/png")
            .body(Cow::Borrowed(BuiltInIcon::from_str(path.as_ref())?.bytes()))
            .map_err(Into::into);
    }

    let path = dunce::canonicalize(PathBuf::from(&*path))?;

    let mimetype = match path.extension().unwrap_or_default().to_str() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => return ipc::response::error("Only png,jpg and svg icons are supported"),
    };

    ipc::response::base()
        .header(CONTENT_TYPE, mimetype)
        .body(std::fs::read(path)?.into())
        .map_err(Into::into)
}
