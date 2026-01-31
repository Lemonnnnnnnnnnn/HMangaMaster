use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use tauri::Manager as TauriManager;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct DownloadTaskDTO {
    pub id: String,
    pub url: String,
    pub status: String,
    pub save_path: String,
    pub start_time: String,
    pub complete_time: String,
    pub updated_at: String,
    pub error: String,
    pub failed_count: i32,
    pub name: String,
    pub progress: Progress,
    #[serde(default = "default_retryable")]
    pub retryable: bool,
}

fn default_retryable() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Progress { pub current: i32, pub total: i32 }

pub struct Manager { data_dir: PathBuf, download_history: Vec<DownloadTaskDTO> }

impl Default for Manager {
    fn default() -> Self {
        let dir = PathBuf::from(".");
        Self { data_dir: dir, download_history: vec![] }
    }
}

impl Manager {
    pub fn set_dir_from_app(&mut self, app: &tauri::AppHandle) -> anyhow::Result<()> {
        self.data_dir = default_data_dir(app)?;
        // 切换目录后尝试加载
        let _ = self.load();
        Ok(())
    }

    pub fn get_history(&self) -> Vec<DownloadTaskDTO> {
        let mut history = self.download_history.clone();
        // 倒序排列
        history.sort_by_key(|d| d.complete_time.clone());
        history.reverse();
        history
    }
    pub fn add_record(&mut self, d: DownloadTaskDTO) { self.download_history.push(d); let _ = self.save(); }
    pub fn clear(&mut self) { self.download_history.clear(); let _ = self.save(); }

    fn save(&self) -> anyhow::Result<()> {
        let path = self.data_dir.join("download_history.json");
        if let Some(p) = path.parent() { std::fs::create_dir_all(p)?; }
        let data = serde_json::to_string_pretty(&self.download_history)?;
        fs::write(path, data)?;
        Ok(())
    }
    fn load(&mut self) -> anyhow::Result<()> {
        let path = self.data_dir.join("download_history.json");
        if !path.exists() { return Ok(()); }
        let data = fs::read_to_string(path)?;
        self.download_history = serde_json::from_str(&data).unwrap_or_default();
        Ok(())
    }
}

fn default_data_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    #[allow(deprecated)]
    let base = app
        .path()
        .app_data_dir()
        .unwrap_or(std::env::temp_dir());
    Ok(base)
}


