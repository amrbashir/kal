pub mod iterator;
pub mod path;
pub mod shell;
pub mod string;

pub use self::iterator::*;
pub use self::path::*;
pub use self::shell::*;
pub use self::string::*;

#[cfg(windows)]
pub fn system_accent_color() -> Option<String> {
    use windows::UI::ViewManagement::*;

    let settings = UISettings::new().ok()?;
    let color = settings.GetColorValue(UIColorType::AccentLight2).ok()?;
    let color_rgb = format!("rgba({},{},{},{})", color.R, color.G, color.B, color.A);
    Some(color_rgb)
}
