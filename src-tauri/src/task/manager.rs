use parking_lot::RwLock;
// use core::fmt;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use crate::download::{Config as DownloadConfig, Downloader};
use crate::history;
use crate::request::Client as RequestClient;
use rr::HeaderMap;

use super::{Progress, Task, TaskStatus};

pub struct TaskManager {
    pub tasks: Arc<RwLock<HashMap<String, Task>>>,
    pub download_concurrency: usize,
}

impl Default for TaskManager {
    fn default() -> Self {
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            download_concurrency: 8,
        }
    }
}

fn now_str() -> String {
    chrono::Utc::now().to_rfc3339()
}

impl TaskManager {
    pub fn create_or_start(&self, task_id: &str, url: &str, total: i32) {
        let mut w = self.tasks.write();
        let mut t = w.remove(task_id).unwrap_or_default();
        t.id = task_id.to_string();
        t.url = url.to_string();
        t.status = TaskStatus::Parsing;
        t.progress = Progress { current: 0, total };
        t.start_time = now_str();
        t.updated_at = t.start_time.clone();
        w.insert(task_id.to_string(), t);
    }

    pub fn set_status_downloading(&self, task_id: &str, total: i32) {
        let mut w = self.tasks.write();
        if let Some(t) = w.get_mut(task_id) {
            t.status = TaskStatus::Running;
            t.progress.total = total;
            t.updated_at = now_str();
        }
    }

    pub fn set_name_and_path(&self, task_id: &str, name: &str, save_path: &str) {
        let mut w = self.tasks.write();
        if let Some(t) = w.get_mut(task_id) {
            t.name = name.to_string();
            t.save_path = save_path.to_string();
            t.updated_at = now_str();
        }
    }

    pub fn set_name(&self, task_id: &str, name: &str) {
        let mut w = self.tasks.write();
        if let Some(t) = w.get_mut(task_id) {
            t.name = name.to_string();
            t.updated_at = now_str();
        }
    }

    pub fn set_failed(&self, task_id: &str, error: &str) {
        let mut w = self.tasks.write();
        if let Some(t) = w.get_mut(task_id) {
            t.status = TaskStatus::Failed;
            t.error = error.to_string();
            t.complete_time = now_str();
            t.updated_at = t.complete_time.clone();
        }
    }

    pub fn set_cancelled(&self, task_id: &str) {
        let mut w = self.tasks.write();
        if let Some(t) = w.get_mut(task_id) {
            t.status = TaskStatus::Cancelled;
            t.complete_time = now_str();
            t.updated_at = t.complete_time.clone();
        }
    }



    pub fn all(&self) -> Vec<Task> {
        self.tasks.read().values().cloned().collect()
    }
    pub fn active(&self) -> Vec<Task> {
        self.tasks
            .read()
            .values()
            .filter(|t| t.status == TaskStatus::Running || t.status == TaskStatus::Parsing)
            .cloned()
            .collect()
    }
    pub fn by_id(&self, task_id: &str) -> Option<Task> {
        self.tasks.read().get(task_id).cloned()
    }
    pub fn clear_non_active(&self) {
        self.tasks
            .write()
            .retain(|_, t| t.status == TaskStatus::Running || t.status == TaskStatus::Parsing)
    }

    pub fn start_batch(
        &self,
        app: AppHandle,
        task_id: String,
        urls: Vec<String>,
        paths: Vec<std::path::PathBuf>,
        client: RequestClient,
        token_opt: Option<CancellationToken>,
        default_headers: Option<HeaderMap>,
    ) -> CancellationToken {
        use futures_util::stream;
        use futures_util::StreamExt;
        let concurrency = self.download_concurrency;
        // 将请求客户端的限流与期望并发对齐，避免内部信号量限制导致并发达不到预期
        let client = client.with_limit(concurrency);
        let downloader =
            Downloader::new_with_headers(client, DownloadConfig::default(), default_headers);
        let token = token_opt.unwrap_or_else(CancellationToken::new);
        let total = urls.len() as i32;
        {
            let mut w = self.tasks.write();
            let t = w.entry(task_id.clone()).or_default();
            t.progress.total = total;
            t.status = TaskStatus::Running;
            t.updated_at = now_str();
        }
        let ct = token.clone();
        let tm = self.tasks.clone();
        tauri::async_runtime::spawn(async move {
            let mut stream = stream::iter(urls.into_iter().zip(paths.into_iter()).map(|(u, p)| {
                let d = downloader.clone();
                let cancel = ct.clone();
                async move {
                    if cancel.is_cancelled() {
                        let res: anyhow::Result<()> = Err(anyhow::anyhow!("cancelled"));
                        return res;
                    }
                    let res = d.download_file(&u, &p).await;
                    res
                }
            }))
            .buffer_unordered(concurrency);

            let mut current: i32 = 0;
            let mut failed_count: i32 = 0;
            while let Some(res) = stream.next().await {
                current += 1;
                if res.is_err() {
                    failed_count += 1;
                }
                let mut w = tm.write();
                if let Some(t) = w.get_mut(&task_id) {
                    t.progress.current = current;
                    t.progress.total = total;
                    t.failed_count = failed_count;
                    t.updated_at = now_str();
                }
                if res.is_err() {
                    if let Some(t) = w.get_mut(&task_id) {
                        if t.error.is_empty() {
                            t.error = format!("{:?}", res.err());
                        }
                    }
                }
            }
            // 更新状态并写入历史
            let (status_str, error_msg);
            {
                let mut w = tm.write();
                if let Some(t) = w.get_mut(&task_id) {
                    if ct.is_cancelled() {
                        t.status = TaskStatus::Cancelled;
                    } else if t.failed_count == 0 {
                        t.status = TaskStatus::Completed;
                    } else if t.failed_count == t.progress.total {
                        t.status = TaskStatus::Failed;
                    } else {
                        t.status = TaskStatus::PartialFailed;
                    }
                    t.complete_time = now_str();
                    t.updated_at = t.complete_time.clone();
                    status_str = match t.status {
                        TaskStatus::Pending => "pending".to_string(),
                        TaskStatus::Parsing => "parsing".to_string(),
                        TaskStatus::Running => "downloading".to_string(),
                        TaskStatus::Completed => "completed".to_string(),
                        TaskStatus::PartialFailed => "partial_failed".to_string(),
                        TaskStatus::Failed => "failed".to_string(),
                        TaskStatus::Cancelled => "cancelled".to_string(),
                    };
                    error_msg = if t.failed_count > 0 {
                        format!("下载失败 {}/{}。{}", t.failed_count, t.progress.total, t.error)
                    } else {
                        t.error.clone()
                    };
                    // 写历史
                    let dto = history::DownloadTaskDTO {
                        id: t.id.clone(),
                        url: t.url.clone(),
                        status: status_str.clone(),
                        save_path: t.save_path.clone(),
                        start_time: t.start_time.clone(),
                        complete_time: t.complete_time.clone(),
                        updated_at: t.updated_at.clone(),
                        error: error_msg.clone(),
                        failed_count: t.failed_count,
                        name: t.name.clone(),
                        progress: history::Progress {
                            current: t.progress.current,
                            total: t.progress.total,
                        },
                    };
                    drop(w);
                    let mut hm = history::Manager::default();
                    let _ = hm.set_dir_from_app(&app);
                    hm.add_record(dto);
                } else {
                    drop(w);
                    status_str = "failed".to_string();
                    error_msg = "unknown task".to_string();
                }
            }
            // 按状态派发事件
            match status_str.as_str() {
                "completed" => {
                    let _ = app.emit(
                        "download:completed",
                        serde_json::json!({"taskId": task_id , "taskName": tm.read().get(&task_id).unwrap().name}),
                    );
                }
                "cancelled" => {
                    let _ = app.emit(
                        "download:cancelled",
                        serde_json::json!({"taskId": task_id , "taskName": tm.read().get(&task_id).unwrap().name}),
                    );
                }
                _ => {
                    let _ = app.emit("download:failed", serde_json::json!({"taskId": task_id, "taskName": tm.read().get(&task_id).unwrap().name, "message": error_msg}));
                }
            }
        });
        token
    }
}
