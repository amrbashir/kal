use serde::{Deserialize, Serialize};

use crate::webview_window::WebviewWindow;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Vibrancy {
    Mica,
    MicaLight,
    MicaDark,
    Tabbed,
    TabbedLight,
    TabbedDark,
    Acrylic,
    Blur,
}

impl Vibrancy {
    pub fn apply(&self, window: &WebviewWindow) -> anyhow::Result<()> {
        match self {
            #[cfg(windows)]
            Vibrancy::Mica => window_vibrancy::apply_mica(window, None),
            #[cfg(windows)]
            Vibrancy::MicaLight => window_vibrancy::apply_mica(window, Some(false)),
            #[cfg(windows)]
            Vibrancy::MicaDark => window_vibrancy::apply_mica(window, Some(true)),
            #[cfg(windows)]
            Vibrancy::Tabbed => window_vibrancy::apply_tabbed(window, None),
            #[cfg(windows)]
            Vibrancy::TabbedLight => window_vibrancy::apply_tabbed(window, Some(false)),
            #[cfg(windows)]
            Vibrancy::TabbedDark => window_vibrancy::apply_tabbed(window, Some(true)),
            #[cfg(windows)]
            Vibrancy::Acrylic => window_vibrancy::apply_acrylic(window, None),
            #[cfg(windows)]
            Vibrancy::Blur => window_vibrancy::apply_blur(window, None),

            #[allow(unreachable_patterns)]
            _ => Ok(()),
        }
        .map_err(Into::into)
    }
}
