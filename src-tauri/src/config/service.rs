use std::sync::Arc;
use std::path::PathBuf;
use parking_lot::RwLock;
use tauri::Manager as TauriManager;
use crate::config::{Config, repository::{ConfigRepository, FileConfigRepository}, parser_config::{ParserConfig, ParserConfigManager}};

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
    fn set_parser_config_auto_save(&mut self, parser_name: &str, config: ParserConfig) -> anyhow::Result<()>;
    fn get_all_parser_configs(&self) -> std::collections::HashMap<String, ParserConfig>;
}

/// 应用配置服务实现
pub struct AppConfigService {
    config_path: PathBuf,
    repository: Box<dyn ConfigRepository>,
    parser_config_manager: Arc<RwLock<ParserConfigManager>>,
}

impl Default for AppConfigService {
    fn default() -> Self {
        Self {
            config_path: PathBuf::new(),
            repository: Box::new(FileConfigRepository::new(PathBuf::new())),
            parser_config_manager: Arc::new(RwLock::new(ParserConfigManager::new())),
        }
    }
}

impl AppConfigService {
    pub fn new(
        config_path: PathBuf,
        repository: Box<dyn ConfigRepository>,
        parser_config_manager: Arc<RwLock<ParserConfigManager>>,
    ) -> Self {
        Self {
            config_path,
            repository,
            parser_config_manager,
        }
    }

    pub fn set_path_from_app(&mut self, app: &tauri::AppHandle) -> anyhow::Result<()> {
        self.config_path = default_config_path(app)?;
        let repository = Box::new(FileConfigRepository::new(self.config_path.clone()));
        self.repository = repository;
        Ok(())
    }

    pub fn load_or_default(&mut self) -> anyhow::Result<()> {
        // 加载配置并初始化解析器配置管理器
        let config = self.load()?;
        if let Some(parser_configs) = &config.parser_configs {
            let parser_config_manager = Arc::new(RwLock::new(ParserConfigManager::new()));
            for (name, config) in parser_configs {
                parser_config_manager.write().set_config(name, config.clone());
            }

            // 更新配置服务中的解析器配置管理器
            self.parser_config_manager = parser_config_manager;
        }
        Ok(())
    }

    pub fn get_config_path(&self) -> String {
        self.config_path.to_string_lossy().to_string()
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

    fn set_parser_config_auto_save(&mut self, parser_name: &str, config: ParserConfig) -> anyhow::Result<()> {
        self.set_parser_config(parser_name, config)
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
