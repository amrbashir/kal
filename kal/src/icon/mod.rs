use std::borrow::Cow;
use std::path::PathBuf;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};
use wry::http::header::CONTENT_TYPE;
use wry::http::{Request, Response};
use wry::WebViewId;

use crate::windowing::ipc;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub enum IconType {
    Path,
    Svg,
    BuiltinIcon,
    Url,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Icon<'a> {
    pub data: Cow<'a, str>,
    pub r#type: IconType,
}

impl<'a> Icon<'a> {
    pub fn path(data: Cow<'a, str>) -> Self {
        Self {
            data,
            r#type: IconType::Path,
        }
    }

    pub fn builtin(data: Cow<'a, str>) -> Self {
        Self {
            data,
            r#type: IconType::BuiltinIcon,
        }
    }

    pub fn svg(data: Cow<'a, str>) -> Self {
        Self {
            data,
            r#type: IconType::Svg,
        }
    }
}

#[derive(EnumString, AsRefStr, Clone, Copy)]
pub enum BuiltinIcon {
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

impl BuiltinIcon {
    pub fn icon(&self) -> Icon<'_> {
        match self {
            Self::Shutdown => Icon::svg(include_str!("./Shutdown.svg").into()),
            Self::Restart => Icon::svg(include_str!("./Restart.svg").into()),
            Self::SignOut => Icon::svg(include_str!("./Signout.svg").into()),
            Self::Hibernate => Icon::svg(include_str!("./Hibernate.svg").into()),
            Self::Sleep => Icon::svg(include_str!("./Sleep.svg").into()),
            Self::Directory => Icon::svg(include_str!("./Folder.svg").into()),
            Self::Lock => Icon::svg(include_str!("./Lock.svg").into()),
            Self::Calculator => Icon::svg(include_str!("./Calculator.svg").into()),
            Self::Workflow => Icon::svg(include_str!("./Workflow.svg").into()),
            _ => Icon::builtin(Cow::Borrowed(self.as_ref())),
        }
    }

    pub fn bytes(&self) -> &'static [u8] {
        match self {
            Self::Url => include_bytes!("./url.png"),
            Self::Shell => include_bytes!("./shell.png"),
            _ => unreachable!(),
        }
    }
}

/// `kalicon://` protocol
#[tracing::instrument]
pub fn kalicon_protocol<'a>(
    _webview_id: WebViewId,
    request: Request<Vec<u8>>,
) -> Result<Response<Cow<'a, [u8]>>, anyhow::Error> {
    let path = &request.uri().path()[1..];
    let path = percent_encoding::percent_decode_str(path).decode_utf8()?;

    let query = request.uri().query();
    if query.map(|q| q.contains("type=builtin")).unwrap_or(false) {
        return Response::builder()
            .header(CONTENT_TYPE, "image/png")
            .body(Cow::Borrowed(BuiltinIcon::from_str(path.as_ref())?.bytes()))
            .map_err(Into::into);
    }

    let path = dunce::canonicalize(PathBuf::from(&*path))?;

    let mimetype = match path.extension().unwrap_or_default().to_str() {
        Some("png") => "image/png",
        Some("jpg") | Some("jpeg") => "image/jpeg",
        Some("svg") => "image/svg+xml",
        _ => return ipc::error_response("Only png,jpg and svg icons are supported"),
    };

    ipc::base_response()
        .header(CONTENT_TYPE, mimetype)
        .body(std::fs::read(path)?.into())
        .map_err(Into::into)
}
