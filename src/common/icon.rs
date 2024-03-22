use std::borrow::Cow;

use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Icon<'a> {
    pub data: Cow<'a, str>,
    pub kind: IconKind,
}

impl<'a> Icon<'a> {
    pub fn path(data: Cow<'a, str>) -> Self {
        Self {
            data,
            kind: IconKind::Path,
        }
    }

    pub fn default(data: Cow<'a, str>) -> Self {
        Self {
            data,
            kind: IconKind::Default,
        }
    }

    pub fn svg(data: Cow<'a, str>) -> Self {
        Self {
            data,
            kind: IconKind::Svg,
        }
    }
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum IconKind {
    Path,
    Svg,
    Default,
}

pub enum Defaults {
    Directory,
    Url,
    File,
    Shell,
    Shutdown,
    Restart,
    SignOut,
    Hibernate,
    Sleep,
    Lock,
}

impl Defaults {
    pub fn path(&self) -> &str {
        match self {
            Defaults::Url => "icons/defaults/url",
            Defaults::Shell => "icons/defaults/shell",
            _ => unreachable!(),
        }
    }

    pub fn icon(&self) -> Icon<'_> {
        match self {
            Defaults::Shutdown => Icon::svg(include_str!("./icons/shutdown.svg").into()),
            Defaults::Restart => Icon::svg(include_str!("./icons/restart.svg").into()),
            Defaults::SignOut => Icon::svg(include_str!("./icons/signout.svg").into()),
            Defaults::Hibernate => Icon::svg(include_str!("./icons/hibernate.svg").into()),
            Defaults::Sleep => Icon::svg(include_str!("./icons/sleep.svg").into()),
            Defaults::Directory => Icon::svg(include_str!("./icons/folder.svg").into()),
            Defaults::File => Icon::svg(include_str!("./icons/file.svg").into()),
            Defaults::Lock => Icon::svg(include_str!("./icons/lock.svg").into()),
            _ => Icon::default(self.path().into()),
        }
    }

    pub fn bytes(path: &str) -> &'static [u8] {
        let icon = path.split('/').next_back().unwrap();
        match icon {
            // TODO: replace with svgs
            "url" => include_bytes!("./icons/url.png"),
            "shell" => include_bytes!("./icons/shell.png"),
            _ => &[],
        }
    }
}
