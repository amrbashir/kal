use std::ffi::*;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

/// Type of the icon.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Default, EnumString)]
pub enum IconType {
    /// [`Icon::data`] is a path to icon.
    Path,
    /// [`Icon::data`] is a path to extract icon from.
    ExtractFromPath,
    /// [`Icon::data`] is a combination of two icons where the
    /// the second icon is overlayed on top with half size.
    ///
    /// The data is a string with the format "{bottom}<<>>{top}".
    Overlay,
    /// [`Icon::data`] is an SVG string.
    Svg,
    #[default]
    /// [`Icon::data`] is a [`BuiltinIcon`] variant.
    Builtin,
    /// [`Icon::data`] is a url to an icon.
    Url,
}

/// An icon representation.
#[derive(Serialize, Debug, Clone, Default)]
pub struct Icon {
    /// String content representing the icon data.
    ///
    /// See [`IconType`] for the meaning of the data.
    pub data: String,
    /// The type of the icon, indicating how the `data` field should be interpreted.
    pub r#type: IconType,
}

impl Icon {
    /// Creates a new icon with the given data and type.
    #[inline]
    pub fn new(data: impl Into<String>, r#type: IconType) -> Self {
        Self {
            data: data.into(),
            r#type,
        }
    }

    /// Creates a new icon with the given data and type set to [`IconType::Path`].
    #[inline]
    pub fn path(data: impl Into<String>) -> Self {
        Self::new(data, IconType::Path)
    }

    /// Creates a new icon with the given data and type set to [`IconType::ExtractFromPath`].
    #[inline]
    pub fn extract_path(data: impl Into<String>) -> Self {
        Self::new(data, IconType::ExtractFromPath)
    }

    /// Creates a new icon with the given data and type set to [`IconType::Svg`].
    #[inline]
    pub fn overlay(bottom: impl Into<String>, top: impl Into<String>) -> Self {
        let bottom = bottom.into();
        let top = top.into();
        Self::new(format!("{bottom}<<>>{top}"), IconType::Overlay)
    }

    /// Creates a new icon with the given data and type set to [`IconType::Svg`].
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
    /// Returns the icon corresponding to the given builtin icon type.
    pub fn icon(&self) -> Icon {
        match self {
            Self::FolderOpen => Icon::builtin(include_str!(
                "../../kal/assets/builtin-icons/FolderOpen.svg"
            )),
            Self::BlankFile => {
                Icon::builtin(include_str!("../../kal/assets/builtin-icons/BlankFile.svg"))
            }
            Self::Shutdown => {
                Icon::builtin(include_str!("../../kal/assets/builtin-icons/Shutdown.svg"))
            }
            Self::Restart => {
                Icon::builtin(include_str!("../../kal/assets/builtin-icons/Restart.svg"))
            }
            Self::SignOut => {
                Icon::builtin(include_str!("../../kal/assets/builtin-icons/Signout.svg"))
            }
            Self::Hibernate => {
                Icon::builtin(include_str!("../../kal/assets/builtin-icons/Hibernate.svg"))
            }
            Self::Sleep => Icon::builtin(include_str!("../../kal/assets/builtin-icons/Sleep.svg")),
            Self::Lock => Icon::builtin(include_str!("../../kal/assets/builtin-icons/Lock.svg")),
            Self::Calculator => Icon::builtin(include_str!(
                "../../kal/assets/builtin-icons/Calculator.svg"
            )),
            Self::Workflow => {
                Icon::builtin(include_str!("../../kal/assets/builtin-icons/Workflow.svg"))
            }
            Self::Shell => Icon::builtin(include_str!("../../kal/assets/builtin-icons/Shell.svg")),
            Self::Url => Icon::builtin(include_str!("../../kal/assets/builtin-icons/Url.svg")),
            Self::Admin => Icon::builtin(include_str!("../../kal/assets/builtin-icons/Admin.svg")),
            Self::Error => Icon::builtin(include_str!("../../kal/assets/builtin-icons/Error.svg")),
            Self::Warning => {
                Icon::builtin(include_str!("../../kal/assets/builtin-icons/Warning.svg"))
            }
            Self::Code => Icon::builtin(include_str!("../../kal/assets/builtin-icons/Code.svg")),
        }
    }
}

impl From<BuiltinIcon> for Icon {
    fn from(value: BuiltinIcon) -> Self {
        value.icon()
    }
}

/// Represents an icon resource.
#[derive(Clone, Debug, Copy)]
#[safer_ffi::derive_ReprC]
#[repr(C)]
pub struct CIcon {
    /// Raw data of the icon as a C-style string.
    ///
    /// See [`Self::r#type`] for the type of the meaning of the data.
    pub data: *const c_char,
    /// Type of the icon.
    ///
    /// Possible values are:
    /// - `0 -> Path`: Icon data is the path to icon.
    /// - `1 -> ExtractFromPath`: Icon data is the path to extract icon from.
    /// - `2 -> Overlay`: Icon data is a combination of two icons where the
    ///   the second icon is overlayed on top with half size.
    ///   The data is a string with the format "{bottom}<<>>{top}".
    /// - `3 -> Svg`: Icon data is an SVG string.
    /// - `4 -> Builtin`: Icon data is a BuiltinIcon variant.
    ///   The data is a string representation of the variant.
    ///   Possible values are:
    ///     - `BlankFile`
    ///     - `FolderOpen`
    ///     - `Url`
    ///     - `Shell`
    ///     - `Shutdown`
    ///     - `Restart`
    ///     - `SignOut`
    ///     - `Hibernate`
    ///     - `Sleep`
    ///     - `Lock`
    ///     - `Calculator`
    ///     - `Workflow`
    ///     - `Admin`
    ///     - `Error`
    ///     - `Warning`
    ///     - `Code`
    /// - `5 -> Url`: Icon data is a url to an icon.
    pub r#type: u8,
}

impl From<CIcon> for Icon {
    fn from(c_icon: CIcon) -> Self {
        Self {
            data: unsafe {
                std::ffi::CStr::from_ptr(c_icon.data)
                    .to_string_lossy()
                    .into_owned()
            },
            r#type: unsafe { std::mem::transmute(c_icon.r#type) },
        }
    }
}
