use anyhow::Ok;
use kal_config::Config;

use crate::icon::BuiltinIcon;
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
            icon: BuiltinIcon::Calculator.into(),
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
    fn new(_: &Config) -> Self {
        Self
    }

    fn name(&self) -> &'static str {
        Self::NAME
    }

    fn default_plugin_config(&self) -> kal_config::PluginConfig {
        kal_config::PluginConfig {
            enabled: Some(true),
            include_in_global_results: Some(true),
            direct_activation_command: Some("=".into()),
            inner: None,
        }
    }

    async fn query(
        &mut self,
        query: &str,
        _matcher: &mut crate::fuzzy_matcher::Matcher,
    ) -> anyhow::Result<PluginQueryOutput> {
        let mut ctx = sci_calc::context::Context::new();

        let result = sci_calc::calculate(&query, &mut ctx)
            .map_err(|e| anyhow::anyhow!("{e}"))?
            .to_string();

        let item = self.item(result);

        Ok(PluginQueryOutput::One(item))
    }
}
