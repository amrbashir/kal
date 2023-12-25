use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Icon {
    pub data: String,
    pub r#type: IconType,
}

#[derive(Serialize, Debug, Clone, PartialEq, Eq)]
pub enum IconType {
    Path,
    // TODO: remove this allow
    #[allow(unused)]
    Svg,
    Default,
}

pub enum Defaults {
    Folder,
    Url,
    File,
    Shell,
}

impl Defaults {
    pub fn path(&self) -> &str {
        match self {
            Defaults::Folder => "icons/defaults/folder",
            Defaults::Url => "icons/defaults/url",
            Defaults::File => "icons/defaults/file",
            Defaults::Shell => "icons/defaults/shell",
        }
    }

    pub fn icon(&self) -> Icon {
        Icon {
            data: match self {
                Defaults::Folder => self.path().to_string(),
                Defaults::Url => self.path().to_string(),
                Defaults::File => self.path().to_string(),
                Defaults::Shell => self.path().to_string(),
            },
            r#type: IconType::Default,
        }
    }

    pub fn bytes(path: &str) -> &'static [u8] {
        let icon = path.split('/').next_back().unwrap();
        match icon {
            // TODO: replace with svgs
            #[cfg(windows)]
            "folder" => include_bytes!("./icons/windows/folder.png"),
            #[cfg(windows)]
            "file" => include_bytes!("./icons/windows/file.png"),
            #[cfg(windows)]
            "url" => include_bytes!("./icons/windows/url.png"),
            #[cfg(windows)]
            "shell" => include_bytes!("./icons/windows/shell.png"),
            _ => &[],
        }
    }
}
