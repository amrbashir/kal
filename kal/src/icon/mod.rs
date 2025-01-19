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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IconType {
    Path,
    Svg,
    #[default]
    BuiltIn,
    Url,
}

#[derive(Serialize, Debug, Clone, Default)]
pub struct Icon {
    pub data: String,
    pub r#type: IconType,
}

impl Icon {
    #[inline]
    pub fn new(data: impl Into<String>, r#type: IconType) -> Self {
        Self {
            data: data.into(),
            r#type,
        }
    }

    #[inline]
    pub fn path(data: impl Into<String>) -> Self {
        Self::new(data, IconType::Path)
    }

    #[inline]
    pub fn builtin(data: impl Into<String>) -> Self {
        Self::new(data, IconType::BuiltIn)
    }
}

impl<'de> Deserialize<'de> for Icon {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        pub struct IconDeser {
            pub data: String,
            pub r#type: IconType,
        }

        let mut icon = IconDeser::deserialize(deserializer)?;
        if icon.r#type == IconType::BuiltIn {
            let builtin = BuiltInIcon::from_str(&icon.data).map_err(serde::de::Error::custom)?;
            icon.data = builtin.icon().data;
        };

        Ok(Self {
            data: icon.data,
            r#type: icon.r#type,
        })
    }
}

#[derive(EnumString, AsRefStr, Clone, Copy)]
pub enum BuiltInIcon {
    BlankFile,
    Folder,
    FolderOpen,
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
    Admin,
    Error,
    Warning,
}

impl BuiltInIcon {
    pub fn icon(&self) -> Icon {
        match self {
            Self::Folder => Icon::builtin(include_str!("./built-in-icons/Folder.svg")),
            Self::FolderOpen => Icon::builtin(include_str!("./built-in-icons/FolderOpen.svg")),
            Self::BlankFile => Icon::builtin(include_str!("./built-in-icons/BlankFile.svg")),
            Self::Shutdown => Icon::builtin(include_str!("./built-in-icons/Shutdown.svg")),
            Self::Restart => Icon::builtin(include_str!("./built-in-icons/Restart.svg")),
            Self::SignOut => Icon::builtin(include_str!("./built-in-icons/Signout.svg")),
            Self::Hibernate => Icon::builtin(include_str!("./built-in-icons/Hibernate.svg")),
            Self::Sleep => Icon::builtin(include_str!("./built-in-icons/Sleep.svg")),
            Self::Lock => Icon::builtin(include_str!("./built-in-icons/Lock.svg")),
            Self::Calculator => Icon::builtin(include_str!("./built-in-icons/Calculator.svg")),
            Self::Workflow => Icon::builtin(include_str!("./built-in-icons/Workflow.svg")),
            Self::Shell => Icon::builtin(include_str!("./built-in-icons/Shell.svg")),
            Self::Url => Icon::builtin(include_str!("./built-in-icons/Url.svg")),
            Self::Admin => Icon::builtin(include_str!("./built-in-icons/Admin.svg")),
            Self::Error => Icon::builtin(include_str!("./built-in-icons/Error.svg")),
            Self::Warning => Icon::builtin(include_str!("./built-in-icons/Warning.svg")),
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
        _ => anyhow::bail!("Only png, jpg and svg icons are supported"),
    };

    ipc::response::base()
        .header(CONTENT_TYPE, mimetype)
        .body(std::fs::read(path)?.into())
        .map_err(Into::into)
}
