use serde::{Deserialize, Serialize};

pub mod parser_config;
pub mod repository;
pub mod service;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub libraries: Vec<String>,
    pub output_dir: String,
    pub proxy_url: String,
    pub active_library: String,
    pub parser_configs: Option<std::collections::HashMap<String, parser_config::ParserConfig>>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            libraries: Vec::new(),
            output_dir: String::new(),
            proxy_url: String::new(),
            active_library: String::new(),
            parser_configs: None,
        }
    }
}

