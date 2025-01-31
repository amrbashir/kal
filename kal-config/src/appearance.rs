use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Vibrancy effects.
///
/// Default: [`Vibrancy::Mica`]
#[derive(Serialize, Deserialize, Default, Debug, Clone, Copy)]
pub enum Vibrancy {
    /// Mica effect, Windows 11 only.
    #[default]
    Mica,
    /// Alternate mica effect, Windows 11 only.
    Tabbed,
    /// Acrylic effect. Windows 11 only for now.
    Acrylic,
}

/// Appearance configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct AppearanceConfig {
    /// Window width.
    ///
    /// Default: `650`
    #[serde(default = "default_window_width")]
    pub window_width: u32,
    /// Input height.
    ///
    /// Default: `65`
    #[serde(default = "default_input_height")]
    pub input_height: u32,
    /// Gap between input and result items.
    ///
    /// Default: `16`
    #[serde(default = "default_input_items_gap")]
    pub input_items_gap: u32,
    /// Number of items to show before scrolling.
    ///
    /// Default: `8`
    #[serde(default = "default_max_items")]
    pub max_items: u32,
    /// Result item height.
    ///
    /// Default: `55`
    #[serde(default = "default_item_height")]
    pub item_height: u32,
    /// Gap between result items.
    ///
    /// Default: `4`
    #[serde(default = "default_item_gap")]
    pub item_gap: u32,
    #[serde(default = "default_true")]
    /// Whether the window is transparent or not.
    ///
    /// Default: `true`
    pub transparent: bool,
    /// Whether the window has shadows or not.
    ///
    /// Default: `true`
    #[serde(default = "default_true")]
    pub shadows: bool,
    /// The window vibrancy effects.
    ///
    /// Default: [`Vibrancy::Mica`]
    #[serde(default = "default_vibrancy")]
    pub vibrancy: Option<Vibrancy>,
    /// A path to a custom CSS file.
    ///
    /// Default: None
    pub custom_css_file: Option<PathBuf>,
}

fn default_window_width() -> u32 {
    650
}
fn default_input_height() -> u32 {
    65
}
fn default_input_items_gap() -> u32 {
    16
}
fn default_max_items() -> u32 {
    8
}
fn default_item_height() -> u32 {
    55
}
fn default_item_gap() -> u32 {
    4
}
fn default_true() -> bool {
    true
}
fn default_vibrancy() -> Option<Vibrancy> {
    Some(Vibrancy::Mica)
}

impl Default for AppearanceConfig {
    fn default() -> Self {
        Self {
            window_width: default_window_width(),
            input_height: default_input_height(),
            input_items_gap: default_input_items_gap(),
            max_items: default_max_items(),
            item_height: default_item_height(),
            item_gap: default_item_gap(),
            transparent: true,
            shadows: true,
            vibrancy: default_vibrancy(),
            custom_css_file: None,
        }
    }
}
