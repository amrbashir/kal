use serde::Serialize;

#[derive(Serialize, Debug, Clone)]
pub struct Icon {
    pub value: String,
    pub r#type: IconType,
}

#[derive(Serialize, Debug, Clone)]
pub enum IconType {
    Path,
    Svg,
}
