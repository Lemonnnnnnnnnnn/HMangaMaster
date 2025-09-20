use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager as TauriManager;

pub mod parser_config;

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
    pub config: Config,
    pub config_path: PathBuf,
    pub parser_config: parser_config::ParserConfigManager,
}

impl Default for Manager {
    fn default() -> Self {
        Self {
            config: Config::default(),
            config_path: Default::default(),
            parser_config: parser_config::ParserConfigManager::new(),
        }
    }
}

impl Manager {
    pub fn set_path_from_app(&mut self, app: &tauri::AppHandle) -> anyhow::Result<()> {
        self.config_path = default_config_path(app)?;
        Ok(())
    }

    pub fn load_or_default(&mut self) -> anyhow::Result<()> {
        if self.config_path.exists() {
            let data = fs::read_to_string(&self.config_path)?;
            self.config = serde_json::from_str(&data).unwrap_or_default();

            // 从持久化配置初始化 ParserConfigManager
            if let Some(parser_configs) = &self.config.parser_configs {
                for (name, config) in parser_configs {
                    self.parser_config.set_config(name, config.clone());
                }
            }
        } else {
            self.save()?;
        }
        Ok(())
    }

    pub fn save(&mut self) -> anyhow::Result<()> {
        // 从 ParserConfigManager 获取最新配置并同步到 Config
        let mut parser_configs = self.config.parser_configs.take().unwrap_or_default();
        for (name, config) in self.parser_config.get_all_configs() {
            parser_configs.insert(name.clone(), config.clone());
        }
        self.config.parser_configs = Some(parser_configs);

        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string(&self.config)?;
        fs::write(&self.config_path, data)?;
        Ok(())
    }

    pub fn get_libraries(&self) -> Vec<String> {
        self.config.libraries.clone()
    }
    pub fn get_active_library(&self) -> String {
        self.config.active_library.clone()
    }
    pub fn set_active_library(&mut self, lib: String) -> anyhow::Result<()> {
        self.config.active_library = lib;
        self.save()
    }
    pub fn get_output_dir(&self) -> String {
        self.config.output_dir.clone()
    }
    pub fn set_output_dir(&mut self, dir: String) -> anyhow::Result<()> {
        self.config.output_dir = dir;
        self.save()
    }
    pub fn get_proxy(&self) -> String {
        self.config.proxy_url.clone()
    }
    pub fn set_proxy(&mut self, proxy: String) -> anyhow::Result<()> {
        self.config.proxy_url = proxy;
        self.save()
    }

    // Parser 配置相关方法 - 自动持久化
    pub fn set_parser_config_auto_save(&mut self, parser_name: &str, config: parser_config::ParserConfig) -> anyhow::Result<()> {
        self.parser_config.set_config(parser_name, config);
        self.save()
    }

    pub fn add_library(&mut self, dir: String) -> anyhow::Result<()> {
        if !self.config.libraries.iter().any(|d| d == &dir) {
            self.config.libraries.push(dir);
            self.save()?;
        }
        Ok(())
    }

    pub fn get_config_path(&self) -> String {
        self.config_path.to_string_lossy().to_string()
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
