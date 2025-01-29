use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

mod extract;
mod service;

pub use self::extract::*;
pub use self::service::*;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, EnumString)]
pub enum IconType {
    /// [`Icon::data`] is the path to icon.
    Path,
    /// [`Icon::data`] is the path to extract icon from.
    ExtractFromPath,
    /// [`Icon::data`] is a combination of two icons where the
    /// the second icon is overlayed on top with half size.
    Overlay,
    /// [`Icon::data`] is an SVG string.
    Svg,
    #[default]
    /// [`Icon::data`] is a [`BuiltinIcon`] variant.
    Builtin,
    /// [`Icon::data`] is a url to an icon.
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
    pub fn extract_path(data: impl Into<String>) -> Self {
        Self::new(data, IconType::ExtractFromPath)
    }

    #[inline]
    pub fn overlay(bottom: impl Into<String>, top: impl Into<String>) -> Self {
        let bottom = bottom.into();
        let top = top.into();
        Self::new(format!("{bottom}<<>>{top}"), IconType::Overlay)
    }

    #[inline]
    pub fn builtin(data: impl Into<String>) -> Self {
        Self::new(data, IconType::Builtin)
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
        if icon.r#type == IconType::Builtin {
            let builtin = BuiltinIcon::from_str(&icon.data).map_err(serde::de::Error::custom)?;
            icon.data = builtin.icon().data;
        };

        Ok(Self {
            data: icon.data,
            r#type: icon.r#type,
        })
    }
}

#[derive(EnumString, AsRefStr, Clone, Copy)]
pub enum BuiltinIcon {
    BlankFile,
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
    Code,
}

impl BuiltinIcon {
    pub fn icon(&self) -> Icon {
        match self {
            Self::FolderOpen => Icon::builtin(include_str!("./builtin-icons/FolderOpen.svg")),
            Self::BlankFile => Icon::builtin(include_str!("./builtin-icons/BlankFile.svg")),
            Self::Shutdown => Icon::builtin(include_str!("./builtin-icons/Shutdown.svg")),
            Self::Restart => Icon::builtin(include_str!("./builtin-icons/Restart.svg")),
            Self::SignOut => Icon::builtin(include_str!("./builtin-icons/Signout.svg")),
            Self::Hibernate => Icon::builtin(include_str!("./builtin-icons/Hibernate.svg")),
            Self::Sleep => Icon::builtin(include_str!("./builtin-icons/Sleep.svg")),
            Self::Lock => Icon::builtin(include_str!("./builtin-icons/Lock.svg")),
            Self::Calculator => Icon::builtin(include_str!("./builtin-icons/Calculator.svg")),
            Self::Workflow => Icon::builtin(include_str!("./builtin-icons/Workflow.svg")),
            Self::Shell => Icon::builtin(include_str!("./builtin-icons/Shell.svg")),
            Self::Url => Icon::builtin(include_str!("./builtin-icons/Url.svg")),
            Self::Admin => Icon::builtin(include_str!("./builtin-icons/Admin.svg")),
            Self::Error => Icon::builtin(include_str!("./builtin-icons/Error.svg")),
            Self::Warning => Icon::builtin(include_str!("./builtin-icons/Warning.svg")),
            Self::Code => Icon::builtin(include_str!("./builtin-icons/Code.svg")),
        }
    }
}

impl From<BuiltinIcon> for Icon {
    fn from(value: BuiltinIcon) -> Self {
        value.icon()
    }
}
