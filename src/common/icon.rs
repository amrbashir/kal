use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Icon {
    pub data: String,
    pub kind: IconKind,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum IconKind {
    Path,
    Svg,
    Default,
}

pub enum Defaults {
    Folder,
    Url,
    File,
    Shell,
    Shutdown,
    Restart,
    SignOut,
    Hibernate,
    Sleep,
}

impl Defaults {
    pub fn path(&self) -> &str {
        match self {
            Defaults::Folder => "icons/defaults/folder",
            Defaults::Url => "icons/defaults/url",
            Defaults::File => "icons/defaults/file",
            Defaults::Shell => "icons/defaults/shell",
            _ => unreachable!(),
        }
    }

    pub fn icon(&self) -> Icon {
        match self {
            #[cfg(windows)]
            Defaults::Shutdown => Icon {
                data: include_str!("./icons/windows/shutdown.svg").into(),
                kind: IconKind::Svg,
            },
            #[cfg(windows)]
            Defaults::Restart => Icon {
                data: include_str!("./icons/windows/restart.svg").into(),
                kind: IconKind::Svg,
            },
            #[cfg(windows)]
            Defaults::SignOut => Icon {
                data: include_str!("./icons/windows/signout.svg").into(),
                kind: IconKind::Svg,
            },
            #[cfg(windows)]
            Defaults::Hibernate => Icon {
                data: include_str!("./icons/windows/hibernate.svg").into(),
                kind: IconKind::Svg,
            },
            #[cfg(windows)]
            Defaults::Sleep => Icon {
                data: include_str!("./icons/windows/sleep.svg").into(),
                kind: IconKind::Svg,
            },
            #[cfg(windows)]
            Defaults::Folder => Icon {
                data: include_str!("./icons/windows/folder.svg").into(),
                kind: IconKind::Svg,
            },
            #[cfg(windows)]
            Defaults::File => Icon {
                data: include_str!("./icons/windows/file.svg").into(),
                kind: IconKind::Svg,
            },
            _ => Icon {
                data: self.path().to_string(),
                kind: IconKind::Default,
            },
        }
    }

    pub fn bytes(path: &str) -> &'static [u8] {
        let icon = path.split('/').next_back().unwrap();
        match icon {
            // TODO: replace with svgs
            #[cfg(windows)]
            "url" => include_bytes!("./icons/windows/url.png"),
            #[cfg(windows)]
            "shell" => include_bytes!("./icons/windows/shell.png"),
            _ => &[],
        }
    }
}
