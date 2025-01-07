use std::borrow::Cow;

use serde::{Deserialize, Serialize};
use strum::{AsRefStr, EnumString};

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
