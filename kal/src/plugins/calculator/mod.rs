use std::path::Path;

use anyhow::Ok;
use calculator_rs::Calculate;

use crate::config::{Config, GenericPluginConfig};
use crate::icon::BuiltInIcon;
use crate::plugin::PluginQueryOutput;
use crate::result_item::{Action, ResultItem};

#[derive(Debug)]
pub struct Plugin;

impl Plugin {
    const NAME: &str = "Calculator";
    const ID: &str = "Calculator";
    const DESCRIPTION: &str = "Press Enter to copy to clipboard";

    fn item(&self, result: String) -> ResultItem {
        ResultItem {
            id: Self::ID.into(),
            icon: BuiltInIcon::Calculator.icon(),
            primary_text: result,
            secondary_text: Self::DESCRIPTION.into(),
            tooltip: None,
            actions: vec![Action::primary(|item| {
                let mut clipboard = arboard::Clipboard::new()?;
                clipboard.set_text(&item.primary_text).map_err(Into::into)
            })],
            score: 0,
        }
    }
}

#[async_trait::async_trait]
impl crate::plugin::Plugin for Plugin {
    fn new(_: &Config, _: &Path) -> Self {
        Self
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_generic_config(&self) -> GenericPluginConfig {
        GenericPluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: Some("=".into()),
        }
    }

    async fn query(
        &mut self,
        query: &str,
        _matcher: &fuzzy_matcher::skim::SkimMatcherV2,
    ) -> anyhow::Result<PluginQueryOutput> {
        if !query.starts_with(|c: char| c.is_ascii_digit()) {
            return Ok(PluginQueryOutput::None);
        }

        let result = query.calculate()?.to_string();
        let item = self.item(result);

        Ok(PluginQueryOutput::One(item))
    }
}
