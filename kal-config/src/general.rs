use serde::{Deserialize, Serialize};

/// General configuration.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GeneralConfig {
    /// A Hotkey string that consists of one key or modifiers + keys.
    /// For example: `Space` or `Alt+Space` or `Alt+Shift+Space`.
    ///
    /// Default: `Alt+Space`
    #[serde(default = "default_hotkey")]
    pub hotkey: String,
    /// Whether pressing `Tab` will cycle through action buttons or go to next result item.
    ///
    /// Default: `true`
    #[serde(default = "default_true")]
    pub tab_through_action_buttons: bool,
    /// Max number of results to show per query.
    ///
    /// Default: `24`
    #[serde(default = "default_max_results")]
    pub max_results: usize,
}

fn default_hotkey() -> String {
    String::from("Alt+Space")
}

fn default_true() -> bool {
    true
}

fn default_max_results() -> usize {
    24
}

impl Default for GeneralConfig {
    fn default() -> Self {
        Self {
            hotkey: default_hotkey(),
            tab_through_action_buttons: default_true(),
            max_results: default_max_results(),
        }
    }
}
