use serde::{Deserialize, Serialize};
use std::{fs, path};

pub const CONFIG_FILE_NAME: &str = "kal.conf.json";

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    /// A tuple of (Modifier, Key)
    pub hotkey: (String, String),
    pub window_width: u32,
    pub input_height: u32,
    pub results_height: u32,
    pub results_item_height: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            hotkey: ("ControlLeft".into(), "Space".into()),
            window_width: 600,
            input_height: 60,
            results_height: 480,
            results_item_height: 60,
        }
    }
}

impl Config {
    pub fn load_from_path<P: AsRef<path::Path>>(path: P) -> Config {
        let path = path.as_ref();
        let config;
        if path.exists() {
            let config_json =
                fs::read_to_string(path).expect("Failed to read config file content.");
            config = serde_json::from_str::<Config>(&config_json)
                .expect("Failed to deserialize config.");
        } else {
            config = Config::default();
            fs::create_dir_all(
                path.parent()
                    .expect("Failed to get config file parent dir."),
            )
            .expect("Failed to create config parent dir.");
            fs::write(
                path,
                serde_json::to_string(&config)
                    .expect("Failed to serialize Config.")
                    .as_bytes(),
            )
            .expect("Failed to save default config File");
        }

        config
    }
}
