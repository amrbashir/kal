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
}

impl Defaults {
    pub fn path(&self) -> &str {
        match self {
            Defaults::Folder => "icons/defaults/folder",
        }
    }

    pub fn icon(&self) -> Icon {
        Icon {
            data: match self {
                Defaults::Folder => self.path().to_string(),
            },
            r#type: IconType::Default,
        }
    }

    pub fn bytes(path: &str) -> &[u8] {
        let icon = path.split('/').rev().next().unwrap();
        match icon {
            #[cfg(windows)]
            "folder" => include_bytes!("./icons/windows/folder.png"),
            _ => &[],
        }
    }
}
