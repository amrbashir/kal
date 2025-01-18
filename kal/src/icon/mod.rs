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

#[derive(Serialize, Debug, Clone)]
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
    pub fn builtin(data: Cow<'a, str>) -> Self {
        Self::new(data, IconType::BuiltIn)
    }
}

impl<'de> Deserialize<'de> for Icon<'_> {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        pub struct IconDeser<'a> {
            pub data: Cow<'a, str>,
            pub r#type: IconType,
        }

        let mut icon = IconDeser::deserialize(deserializer)?;
        if icon.r#type == IconType::BuiltIn {
            let builtin = BuiltInIcon::from_str(&icon.data).map_err(serde::de::Error::custom)?;
            icon.data = builtin.icon().data.into_owned().into();
        };

        Ok(Self {
            data: icon.data,
            r#type: icon.r#type,
        })
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
            Self::Shutdown => Icon::builtin(include_str!("./built-in-icons/Shutdown.svg").into()),
            Self::Restart => Icon::builtin(include_str!("./built-in-icons/Restart.svg").into()),
            Self::SignOut => Icon::builtin(include_str!("./built-in-icons/Signout.svg").into()),
            Self::Hibernate => Icon::builtin(include_str!("./built-in-icons/Hibernate.svg").into()),
            Self::Sleep => Icon::builtin(include_str!("./built-in-icons/Sleep.svg").into()),
            Self::Directory => Icon::builtin(include_str!("./built-in-icons/Folder.svg").into()),
            Self::Lock => Icon::builtin(include_str!("./built-in-icons/Lock.svg").into()),
            Self::Calculator => {
                Icon::builtin(include_str!("./built-in-icons/Calculator.svg").into())
            }
            Self::Workflow => Icon::builtin(include_str!("./built-in-icons/Workflow.svg").into()),
            Self::Shell => Icon::builtin(include_str!("./built-in-icons/Shell.svg").into()),
            Self::Url => Icon::builtin(include_str!("./built-in-icons/Url.svg").into()),
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
