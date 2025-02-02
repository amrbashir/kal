use serde::Serialize;
use serialize_to_javascript::{Options as JsSerializeOptions, Template as JsTemplate};

#[derive(Clone, Copy, Debug)]
pub struct Color(pub windows::UI::Color);

#[derive(Clone, Copy, Default, Serialize, JsTemplate)]
pub struct SystemAccentColors {
    pub background: Option<Color>,
    pub foreground: Option<Color>,
    pub accent_dark1: Option<Color>,
    pub accent_dark2: Option<Color>,
    pub accent_dark3: Option<Color>,
    pub accent: Option<Color>,
    pub accent_light1: Option<Color>,
    pub accent_light2: Option<Color>,
    pub accent_light3: Option<Color>,
    pub complement: Option<Color>,
}

impl SystemAccentColors {
    const INIT_SCRIPT: &str = r#"(function () {
        window.KAL.systemAccentColors = {
            background: __TEMPLATE_background__,
            foreground: __TEMPLATE_foreground__,
            accent_dark1: __TEMPLATE_accent_dark1__,
            accent_dark2: __TEMPLATE_accent_dark2__,
            accent_dark3: __TEMPLATE_accent_dark3__,
            accent: __TEMPLATE_accent__,
            accent_light1: __TEMPLATE_accent_light1__,
            accent_light2: __TEMPLATE_accent_light2__,
            accent_light3: __TEMPLATE_accent_light3__,
            complement: __TEMPLATE_complement__,
        };
    })()"#;

    pub fn init_script(&self) -> String {
        let js_ser_opts = JsSerializeOptions::default();
        self.render(Self::INIT_SCRIPT, &js_ser_opts)
            .map(|s| s.into_string())
            .unwrap_or_default()
    }
}

#[cfg(windows)]
mod imp {
    use std::ops::Deref;

    use windows::UI::ViewManagement::*;

    use super::*;

    impl From<windows::UI::Color> for Color {
        fn from(value: windows::UI::Color) -> Self {
            Self(value)
        }
    }

    impl Deref for Color {
        type Target = windows::UI::Color;

        fn deref(&self) -> &Self::Target {
            &self.0
        }
    }

    impl Serialize for Color {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: serde::Serializer,
        {
            serializer.serialize_str(&format!(
                "rgb({}, {}, {}, {})",
                self.R, self.G, self.B, self.A
            ))
        }
    }

    impl SystemAccentColors {
        pub fn load() -> anyhow::Result<Self> {
            let settings = UISettings::new()?;

            #[rustfmt::skip]
            let accent = Self {
                background: settings.GetColorValue(UIColorType::Background).ok().map(Into::into),
                foreground: settings.GetColorValue(UIColorType::Foreground).ok().map(Into::into),
                accent_dark1: settings.GetColorValue(UIColorType::AccentDark1).ok().map(Into::into),
                accent_dark2: settings.GetColorValue(UIColorType::AccentDark2).ok().map(Into::into),
                accent_dark3: settings.GetColorValue(UIColorType::AccentDark3).ok().map(Into::into),
                accent: settings.GetColorValue(UIColorType::Accent).ok().map(Into::into),
                accent_light1: settings.GetColorValue(UIColorType::AccentLight1).ok().map(Into::into),
                accent_light2: settings.GetColorValue(UIColorType::AccentLight2).ok().map(Into::into),
                accent_light3: settings.GetColorValue(UIColorType::AccentLight3).ok().map(Into::into),
                complement: settings.GetColorValue(UIColorType::Complement).ok().map(Into::into),
            };

            Ok(accent)
        }
    }
}
