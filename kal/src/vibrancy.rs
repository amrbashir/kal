use serde::{Deserialize, Serialize};

use crate::webview_window::WebViewWindow;

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
    #[cfg(windows)]
    pub fn apply(&self, window: &WebViewWindow) -> anyhow::Result<()> {
        match self {
            Vibrancy::Mica => window_vibrancy::apply_mica(window, None),
            Vibrancy::MicaLight => window_vibrancy::apply_mica(window, Some(false)),
            Vibrancy::MicaDark => window_vibrancy::apply_mica(window, Some(true)),
            Vibrancy::Tabbed => window_vibrancy::apply_tabbed(window, None),
            Vibrancy::TabbedLight => window_vibrancy::apply_tabbed(window, Some(false)),
            Vibrancy::TabbedDark => window_vibrancy::apply_tabbed(window, Some(true)),
            Vibrancy::Acrylic => window_vibrancy::apply_acrylic(window, None),
            Vibrancy::Blur => window_vibrancy::apply_blur(window, None),
        }
        .map_err(Into::into)
    }

    #[cfg(not(windows))]
    pub fn apply(&self, window: &WebViewWindow) -> anyhow::Result<()> {
        Ok(())
    }
}
