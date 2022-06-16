use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Icon {
    pub data: String,
    pub r#type: IconType,
}

#[derive(Serialize, Debug, Clone)]
pub enum IconType {
    Path,
    Svg,
}
