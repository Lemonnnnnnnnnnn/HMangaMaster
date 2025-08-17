use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager as TauriManager;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Config {
    pub libraries: Vec<String>,
    pub output_dir: String,
    pub proxy_url: String,
    pub active_library: String,
}

pub struct Manager {
    pub config: Config,
    pub config_path: PathBuf,
}

impl Default for Manager {
    fn default() -> Self {
        // 初始为相对路径；启动时由 AppHandle 注入正确路径
        let path = PathBuf::from("config.json");
        Self { config: Config::default(), config_path: path }
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
        } else {
            self.save()?;
        }
        Ok(())
    }

    pub fn save(&self) -> anyhow::Result<()> {
        if let Some(parent) = self.config_path.parent() { fs::create_dir_all(parent)?; }
        let data = serde_json::to_string(&self.config)?;
        fs::write(&self.config_path, data)?;
        Ok(())
    }

    pub fn get_libraries(&self) -> Vec<String> { self.config.libraries.clone() }
    pub fn get_active_library(&self) -> String { self.config.active_library.clone() }
    pub fn set_active_library(&mut self, lib: String) -> anyhow::Result<()> {
        self.config.active_library = lib;
        self.save()
    }
    pub fn get_output_dir(&self) -> String { self.config.output_dir.clone() }
    pub fn set_output_dir(&mut self, dir: String) -> anyhow::Result<()> {
        self.config.output_dir = dir;
        self.save()
    }
    pub fn get_proxy(&self) -> String { self.config.proxy_url.clone() }
    pub fn set_proxy(&mut self, proxy: String) -> anyhow::Result<()> {
        self.config.proxy_url = proxy;
        self.save()
    }
    pub fn add_library(&mut self, dir: String) -> anyhow::Result<()> {
        if !self.config.libraries.iter().any(|d| d == &dir) {
            self.config.libraries.push(dir);
            self.save()?;
        }
        Ok(())
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


