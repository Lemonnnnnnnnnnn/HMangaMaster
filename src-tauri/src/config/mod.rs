use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager as TauriManager;
use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::{repository::FileConfigRepository, service::{AppConfigService, ConfigService}, parser_config::ParserConfigManager};

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

pub struct Manager {
    config_path: PathBuf,
    config_service: Arc<RwLock<AppConfigService>>,
}

impl Default for Manager {
    fn default() -> Self {
        let parser_config_manager = Arc::new(RwLock::new(ParserConfigManager::new()));
        let config_service = Arc::new(RwLock::new(AppConfigService::new(
            Box::new(FileConfigRepository::new(PathBuf::new())),
            parser_config_manager,
        )));
        Self {
            config_path: Default::default(),
            config_service,
        }
    }
}

impl Manager {
    pub fn set_path_from_app(&mut self, app: &tauri::AppHandle) -> anyhow::Result<()> {
        self.config_path = default_config_path(app)?;

        // 更新配置服务中的仓储路径
        let repository = Box::new(FileConfigRepository::new(self.config_path.clone()));
        let parser_config_manager = Arc::new(RwLock::new(ParserConfigManager::new()));
        let config_service = Arc::new(RwLock::new(AppConfigService::new(repository, parser_config_manager)));

        self.config_service = config_service;
        Ok(())
    }

    pub fn load_or_default(&mut self) -> anyhow::Result<()> {
        // 加载配置并初始化解析器配置管理器
        let config = self.config_service.read().load()?;
        if let Some(parser_configs) = &config.parser_configs {
            let parser_config_manager = Arc::new(RwLock::new(ParserConfigManager::new()));
            for (name, config) in parser_configs {
                parser_config_manager.write().set_config(name, config.clone());
            }

            // 更新配置服务中的解析器配置管理器
            let repository = Box::new(FileConfigRepository::new(self.config_path.clone()));
            let config_service = Arc::new(RwLock::new(AppConfigService::new(repository, parser_config_manager)));
            self.config_service = config_service;
        }
        Ok(())
    }

    pub fn get_libraries(&self) -> Vec<String> {
        self.config_service.read().get_libraries()
    }

    pub fn get_active_library(&self) -> String {
        self.config_service.read().get_active_library()
    }

    pub fn set_active_library(&mut self, lib: String) -> anyhow::Result<()> {
        self.config_service.write().set_active_library(lib)
    }

    pub fn get_output_dir(&self) -> String {
        self.config_service.read().get_output_dir()
    }

    pub fn set_output_dir(&mut self, dir: String) -> anyhow::Result<()> {
        self.config_service.write().set_output_dir(dir)
    }

    pub fn get_proxy(&self) -> String {
        self.config_service.read().get_proxy()
    }

    pub fn set_proxy(&mut self, proxy: String) -> anyhow::Result<()> {
        self.config_service.write().set_proxy(proxy)
    }

    pub fn set_parser_config_auto_save(&mut self, parser_name: &str, config: parser_config::ParserConfig) -> anyhow::Result<()> {
        self.config_service.write().set_parser_config(parser_name, config)
    }

    pub fn add_library(&mut self, dir: String) -> anyhow::Result<()> {
        self.config_service.write().add_library(dir)
    }

    pub fn get_config_path(&self) -> String {
        self.config_path.to_string_lossy().to_string()
    }

    pub fn get_parser_config(&self, parser_name: &str) -> parser_config::ParserConfig {
        self.config_service.read().get_parser_config(parser_name)
    }

    pub fn get_all_parser_configs(&self) -> std::collections::HashMap<String, parser_config::ParserConfig> {
        self.config_service.read().get_all_parser_configs()
    }
}

fn default_config_path(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    #[allow(deprecated)]
    let base = app
        .path()
        .app_config_dir()
        .unwrap_or_else(|_| app.path().app_data_dir().unwrap_or(std::env::temp_dir()));
    if !base.exists() {
        std::fs::create_dir_all(&base)?;
    }
    Ok(base.join("config.json"))
}
