use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Icon {
    pub data: String,
    pub kind: IconKind,
}

impl Icon {
    pub fn path(data: String) -> Self {
        Self {
            data,
            kind: IconKind::Path,
        }
    }

    pub fn default(data: String) -> Self {
        Self {
            data,
            kind: IconKind::Default,
        }
    }

    pub fn svg(data: String) -> Self {
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
    Folder,
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

    pub fn icon(&self) -> Icon {
        match self {
            Defaults::Shutdown => Icon::svg(include_str!("./icons/shutdown.svg").into()),
            Defaults::Restart => Icon::svg(include_str!("./icons/restart.svg").into()),
            Defaults::SignOut => Icon::svg(include_str!("./icons/signout.svg").into()),
            Defaults::Hibernate => Icon::svg(include_str!("./icons/hibernate.svg").into()),
            Defaults::Sleep => Icon::svg(include_str!("./icons/sleep.svg").into()),
            Defaults::Folder => Icon::svg(include_str!("./icons/folder.svg").into()),
            Defaults::File => Icon::svg(include_str!("./icons/file.svg").into()),
            Defaults::Lock => Icon::svg(include_str!("./icons/lock.svg").into()),
            _ => Icon::default(self.path().to_string()),
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
