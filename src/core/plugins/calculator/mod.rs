use std::path::Path;

use calculator_rs::Calculate;
use serde::{Deserialize, Serialize};

use crate::{
    common::{icon::Defaults, SearchResultItem},
    config::Config,
};

pub struct Plugin {
    clipboard: Option<arboard::Clipboard>,
    last_calculation: String,
}

impl std::fmt::Debug for Plugin {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Plugin")
            .field("clipboard", &"arboard::Clipboard")
            .field("last_calculation", &self.last_calculation)
            .finish()
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct PluginConfig {
    enabled: bool,
}

impl Default for PluginConfig {
    fn default() -> Self {
        Self { enabled: true }
    }
}

impl Plugin {
    const NAME: &'static str = "Calculator";
    const IDENITIFER: &'static str = "Calculator-99999-item";

    #[inline]
    fn item(&self) -> SearchResultItem<'_> {
        SearchResultItem {
            primary_text: self.last_calculation.as_str().into(),
            secondary_text: "Press Enter to copy to clipboard".into(),
            needs_confirmation: false,
            identifier: Self::IDENITIFER.into(),
            icon: Defaults::Calculator.icon(),
            score: 99999, // should always be the first one
        }
    }

    fn try_clipboard(&mut self) {
        if self.clipboard.is_none() {
            self.clipboard = arboard::Clipboard::new().ok();
        }
    }

    fn copy_last_calculation(&mut self) {
        self.try_clipboard();

        if self.clipboard.is_none() {
            return;
        }

        let _ = self
            .clipboard
            .as_mut()
            .unwrap()
            .set_text(&self.last_calculation);
    }
}

impl crate::plugin::Plugin for Plugin {
    fn new(_: &Config, _: &Path) -> anyhow::Result<Self> {
        Ok(Self {
            last_calculation: String::new(),
            clipboard: arboard::Clipboard::new().ok(),
        })
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn results(
        &mut self,
        query: &str,
        _matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<Option<Vec<crate::common::SearchResultItem<'_>>>> {
        if query.starts_with(|c: char| c.is_ascii_digit()) {
            query
                .calculate()
                .map(|res| {
                    self.last_calculation = res.to_string();
                    Some(vec![self.item()])
                })
                .map_err(Into::into)
        } else {
            Ok(None)
        }
    }

    fn execute(&mut self, identifier: &str, _: bool) -> anyhow::Result<()> {
        if identifier == Self::IDENITIFER {
            self.copy_last_calculation()
        }

        Ok(())
    }
}
