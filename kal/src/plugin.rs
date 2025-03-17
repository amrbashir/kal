use kal_config::Config;
use kal_plugin::ResultItem;

use crate::fuzzyer_matcher::FuzzyMatcher;

#[derive(Debug)]
pub struct Plugin {
    pub enabled: bool,
    pub include_in_global_results: bool,
    pub direct_activation_command: Option<String>,
}

impl Plugin {
    pub fn load() -> Self {
        todo!()
    }

    pub fn is_direct_invoke(&self, query: &str) -> bool {
        self.direct_activation_command
            .as_deref()
            .map(|c| query.starts_with(c))
            .unwrap_or(false)
    }

    pub fn direct_invoke_len(&self) -> usize {
        self.direct_activation_command
            .as_ref()
            .map(|c| c.len())
            .unwrap_or_default()
    }

    pub fn update_from_config(&mut self, config: &Config) {
        // let default_c = self.default_plugin_config();

        // match config.plugins.get(self.name()) {
        //     Some(c) => {
        //         self.enabled = c.enabled_or(default_c.enabled);
        //         self.include_in_global_results =
        //             c.include_in_global_results_or(default_c.include_in_global_results);
        //         self.direct_activation_command =
        //             c.direct_activation_command_or(default_c.direct_activation_command.as_ref());
        //     }
        //     None => {
        //         self.enabled = default_c.enabled();
        //         self.include_in_global_results = default_c.include_in_global_results();
        //         self.direct_activation_command = default_c.direct_activation_command();
        //     }
        // };
    }

    /// Convenient method to construct an error [ResultItem] for this plugin.
    pub fn error_item(&self, error: String) -> ResultItem {
        ResultItem {
            id: String::new(),
            icon: crate::icon::BuiltinIcon::Error.into(),
            primary_text: self.name().to_owned(),
            secondary_text: error,
            tooltip: None,
            actions: vec![],
            score: 0,
        }
    }

    pub fn name(&self) -> &str {
        todo!()
    }

    pub fn reload(&self, config: &Config) -> anyhow::Result<()> {
        todo!()
    }

    pub fn query(
        &self,
        query: &str,
        matcher: &mut FuzzyMatcher,
    ) -> anyhow::Result<Vec<ResultItem>> {
        todo!()
    }

    pub fn query_direct(
        &self,
        query: &str,
        matcher: &mut FuzzyMatcher,
    ) -> anyhow::Result<Vec<ResultItem>> {
        todo!()
    }
}
