use std::path::Path;

use calculator_rs::Calculate;

use crate::{config::Config, icon::BuiltinIcon, search_result_item::SearchResultItem};

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

impl Plugin {
    const NAME: &'static str = "Calculator";
    const ID: &'static str = "Calculator-99999-item";

    #[inline]
    fn item(&self) -> SearchResultItem<'_> {
        SearchResultItem {
            primary_text: self.last_calculation.as_str().into(),
            secondary_text: "Press Enter to copy to clipboard".into(),
            needs_confirmation: false,
            id: Self::ID.into(),
            icon: BuiltinIcon::Calculator.icon(),
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
    ) -> anyhow::Result<Option<Vec<SearchResultItem<'_>>>> {
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

    fn execute(&mut self, id: &str, _: bool) -> anyhow::Result<()> {
        if id == Self::ID {
            self.copy_last_calculation()
        }

        Ok(())
    }
}
