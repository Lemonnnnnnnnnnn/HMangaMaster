use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use tauri::Manager as TauriManager;

use crate::task::FailedFile;

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
    #[serde(default)]
    pub failed_files: Vec<FailedFile>,
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
        let mut latest_by_id: HashMap<String, DownloadTaskDTO> = HashMap::new();
        for record in &self.download_history {
            latest_by_id
                .entry(record.id.clone())
                .and_modify(|existing| {
                    if record.complete_time >= existing.complete_time {
                        *existing = record.clone();
                    }
                })
                .or_insert_with(|| record.clone());
        }
        let mut history = latest_by_id.into_values().collect::<Vec<_>>();
        // 倒序排列
        history.sort_by_key(|d| d.complete_time.clone());
        history.reverse();
        history
    }
    pub fn add_record(&mut self, d: DownloadTaskDTO) {
        self.download_history.retain(|record| record.id != d.id);
        self.download_history.push(d);
        let _ = self.save();
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_history_record_without_failed_files_deserializes_with_empty_list() {
        let json = r#"{
            "id": "task-1",
            "url": "https://example.test/gallery",
            "status": "partial_failed",
            "savePath": "D:/manga/gallery",
            "startTime": "2026-04-18T00:00:00Z",
            "completeTime": "2026-04-18T00:01:00Z",
            "updatedAt": "2026-04-18T00:01:00Z",
            "error": "download failed",
            "failedCount": 1,
            "name": "gallery",
            "progress": { "current": 3, "total": 4 },
            "retryable": true
        }"#;

        let record: DownloadTaskDTO = serde_json::from_str(json).unwrap();

        assert!(record.failed_files.is_empty());
    }

    #[test]
    fn get_history_returns_only_latest_record_for_same_task_id() {
        let mut manager = Manager::default();
        manager.download_history = vec![
            DownloadTaskDTO {
                id: "task-1".to_string(),
                status: "partial_failed".to_string(),
                complete_time: "2026-04-18T00:01:00Z".to_string(),
                error: "old failure".to_string(),
                ..DownloadTaskDTO::default()
            },
            DownloadTaskDTO {
                id: "task-1".to_string(),
                status: "completed".to_string(),
                complete_time: "2026-04-18T00:02:00Z".to_string(),
                error: String::new(),
                ..DownloadTaskDTO::default()
            },
        ];

        let history = manager.get_history();

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].id, "task-1");
        assert_eq!(history[0].status, "completed");
    }

    #[test]
    fn add_record_replaces_existing_record_with_same_task_id() {
        let unique_dir = std::env::temp_dir().join(format!(
            "hmanga-history-test-{}",
            chrono::Utc::now().timestamp_nanos_opt().unwrap()
        ));
        let mut manager = Manager {
            data_dir: unique_dir.clone(),
            download_history: vec![DownloadTaskDTO {
                id: "task-1".to_string(),
                status: "partial_failed".to_string(),
                complete_time: "2026-04-18T00:01:00Z".to_string(),
                ..DownloadTaskDTO::default()
            }],
        };

        manager.add_record(DownloadTaskDTO {
            id: "task-1".to_string(),
            status: "completed".to_string(),
            complete_time: "2026-04-18T00:02:00Z".to_string(),
            ..DownloadTaskDTO::default()
        });

        let history = manager.get_history();
        let _ = std::fs::remove_dir_all(unique_dir);

        assert_eq!(history.len(), 1);
        assert_eq!(history[0].status, "completed");
    }
}


