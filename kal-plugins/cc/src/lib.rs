#![allow(unused)]

use std::ffi::*;
use std::mem::ManuallyDrop;

use kal_plugin::{
    define_plugin, BuiltinIcon, CResultItem, Config, Icon, ResultItem, UnsafeMatcherFn,
};

pub struct Plugin {
    s: String,
}

impl Plugin {
    fn new(config: *const Config) -> Self {
        Self {
            s: String::from("asd"),
        }
    }

    fn name(&self) -> &str {
        "CC"
    }

    fn default_plugin_config(&self) -> String {
        String::from("{\"value\": 1}")
    }

    fn reload(&mut self, config: *const Config) {
        println!("Reloading plugin");
        println!(
            "B: {}",
            kal_plugin::rust::config_get_bool(config, "CC", "B")
        );
        println!("S: {}", kal_plugin::rust::config_get_str(config, "CC", "S"));
        println!("I: {}", kal_plugin::rust::config_get_int(config, "CC", "I"));
        println!("Self S: {}", self.s);
    }

    fn query<F: Fn(&str, &str) -> u16>(&self, query: &str, matcher: F) -> Vec<ResultItem> {
        println!("Query: {}", query);

        matcher(query, "asd");

        vec![ResultItem {
            id: "21en.a".to_string(),
            primary_text: "Hello".to_string(),
            secondary_text: "World".to_string(),
            icon: BuiltinIcon::FolderOpen.icon(),
            actions: vec![],
            score: 0,
            tooltip: Some("ToolTip".to_string()),
        }]
    }

    fn query_direct<F: Fn(&str, &str) -> u16>(&self, query: &str, matcher: F) -> Vec<ResultItem> {
        vec![ResultItem {
            id: "21en.a".to_string(),
            primary_text: "Hello".to_string(),
            secondary_text: "World".to_string(),
            icon: BuiltinIcon::FolderOpen.icon(),
            actions: vec![],
            score: 0,
            tooltip: Some("ToolTip".to_string()),
        }]
    }
}

define_plugin!(Plugin);
