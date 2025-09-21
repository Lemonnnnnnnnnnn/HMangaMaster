use std::sync::Arc;
use parking_lot::RwLock;
use crate::config::{Config, repository::ConfigRepository, parser_config::{ParserConfig, ParserConfigManager}};

/// 配置服务接口
pub trait ConfigService {
    fn get_libraries(&self) -> Vec<String>;
    fn add_library(&mut self, path: String) -> anyhow::Result<()>;
    fn get_active_library(&self) -> String;
    fn set_active_library(&mut self, library: String) -> anyhow::Result<()>;
    fn get_output_dir(&self) -> String;
    fn set_output_dir(&mut self, dir: String) -> anyhow::Result<()>;
    fn get_proxy(&self) -> String;
    fn set_proxy(&mut self, proxy: String) -> anyhow::Result<()>;
    fn get_parser_config(&self, parser_name: &str) -> ParserConfig;
    fn set_parser_config(&mut self, parser_name: &str, config: ParserConfig) -> anyhow::Result<()>;
    fn get_all_parser_configs(&self) -> std::collections::HashMap<String, ParserConfig>;
}

/// 应用配置服务实现
pub struct AppConfigService {
    repository: Box<dyn ConfigRepository>,
    parser_config_manager: Arc<RwLock<ParserConfigManager>>,
}

impl AppConfigService {
    pub fn new(
        repository: Box<dyn ConfigRepository>,
        parser_config_manager: Arc<RwLock<ParserConfigManager>>,
    ) -> Self {
        Self {
            repository,
            parser_config_manager,
        }
    }

    pub fn load(&self) -> anyhow::Result<Config> {
        self.repository.load()
    }

    pub fn save(&self, config: &Config) -> anyhow::Result<()> {
        self.repository.save(config)
    }
}

impl ConfigService for AppConfigService {
    fn get_libraries(&self) -> Vec<String> {
        self.load()
            .map(|c| c.libraries)
            .unwrap_or_default()
    }

    fn add_library(&mut self, path: String) -> anyhow::Result<()> {
        let mut config = self.load()?;
        if !config.libraries.iter().any(|d| d == &path) {
            config.libraries.push(path);
            self.save(&config)?;
        }
        Ok(())
    }

    fn get_active_library(&self) -> String {
        self.load()
            .map(|c| c.active_library)
            .unwrap_or_default()
    }

    fn set_active_library(&mut self, library: String) -> anyhow::Result<()> {
        let mut config = self.load()?;
        config.active_library = library;
        self.save(&config)
    }

    fn get_output_dir(&self) -> String {
        self.load()
            .map(|c| c.output_dir)
            .unwrap_or_default()
    }

    fn set_output_dir(&mut self, dir: String) -> anyhow::Result<()> {
        let mut config = self.load()?;
        config.output_dir = dir;
        self.save(&config)
    }

    fn get_proxy(&self) -> String {
        self.load()
            .map(|c| c.proxy_url)
            .unwrap_or_default()
    }

    fn set_proxy(&mut self, proxy: String) -> anyhow::Result<()> {
        let mut config = self.load()?;
        config.proxy_url = proxy;
        self.save(&config)
    }

    fn get_parser_config(&self, parser_name: &str) -> ParserConfig {
        self.parser_config_manager.read().get_config(parser_name)
    }

    fn set_parser_config(&mut self, parser_name: &str, config: ParserConfig) -> anyhow::Result<()> {
        self.parser_config_manager.write().set_config(parser_name, config);
        // 保存解析器配置到主配置中
        let mut main_config = self.load()?;
        let mut parser_configs = main_config.parser_configs.take().unwrap_or_default();
        let parser_config_manager_read = self.parser_config_manager.read();
        let all_configs = parser_config_manager_read.get_all_configs();
        for (name, config) in all_configs {
            parser_configs.insert(name.clone(), config.clone());
        }
        drop(parser_config_manager_read);
        main_config.parser_configs = Some(parser_configs);
        self.save(&main_config)
    }

    fn get_all_parser_configs(&self) -> std::collections::HashMap<String, ParserConfig> {
        self.parser_config_manager.read().get_all_configs().clone()
    }
}
