use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;

use crate::download::{Config as DownloadConfig, Downloader};
use crate::history;
use crate::request::Client as RequestClient;
use reqwest::header::HeaderMap;

use super::{Progress, Task, TaskStatus};

/// Parameters for starting a batch download task
pub struct BatchDownloadParams {
    pub app: AppHandle,
    pub task_id: String,
    pub urls: Vec<String>,
    pub paths: Vec<std::path::PathBuf>,
    pub client: RequestClient,
    pub token_opt: Option<CancellationToken>,
    pub default_headers: Option<HeaderMap>,
    pub concurrency_override: Option<usize>,
}

#[derive(Clone)]
pub struct TaskManager {
    pub tasks: Arc<RwLock<HashMap<String, Task>>>,
    pub download_concurrency: usize,
    pub max_concurrent_tasks: usize,
}

impl Default for TaskManager {
    fn default() -> Self {
        let max_concurrent_tasks = 3; // 默认最多同时运行3个任务
        Self {
            tasks: Arc::new(RwLock::new(HashMap::new())),
            download_concurrency: 8,
            max_concurrent_tasks,
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
            .filter(|t| t.status == TaskStatus::Running || t.status == TaskStatus::Parsing || t.status == TaskStatus::Queued)
            .cloned()
            .collect()
    }
    pub fn by_id(&self, task_id: &str) -> Option<Task> {
        self.tasks.read().get(task_id).cloned()
    }
    pub fn clear_non_active(&self) {
        self.tasks
            .write()
            .retain(|_, t| t.status == TaskStatus::Running || t.status == TaskStatus::Parsing || t.status == TaskStatus::Queued)
    }

    /// 获取当前运行中的任务数量
    pub fn running_task_count(&self) -> usize {
        self.tasks
            .read()
            .values()
            .filter(|t| t.status == TaskStatus::Running || t.status == TaskStatus::Parsing)
            .count()
    }

    /// 获取排队中的任务数量
    pub fn queued_task_count(&self) -> usize {
        self.tasks
            .read()
            .values()
            .filter(|t| t.status == TaskStatus::Queued)
            .count()
    }

    /// 获取下一个排队中的任务（按创建时间排序）
    pub fn get_next_queued_task(&self) -> Option<Task> {
        let tasks = self.tasks.read();
        tasks
            .values()
            .filter(|t| t.status == TaskStatus::Queued)
            .min_by(|a, b| a.start_time.cmp(&b.start_time))
            .cloned()
    }

    /// 将排队中的任务转换为解析状态
    pub fn start_queued_task(&self, task_id: &str) -> bool {
        let mut w = self.tasks.write();
        if let Some(t) = w.get_mut(task_id) {
            if t.status == TaskStatus::Queued {
                t.status = TaskStatus::Parsing;
                t.updated_at = now_str();
                return true;
            }
        }
        false
    }

    /// 设置最大并发任务数
    pub fn set_max_concurrent_tasks(&mut self, max: usize) {
        self.max_concurrent_tasks = max;
    }

    pub fn start_batch_with_concurrency(&self, params: BatchDownloadParams) -> CancellationToken {
        use futures_util::stream;
        use futures_util::StreamExt;
        let concurrency = params.concurrency_override.unwrap_or(self.download_concurrency);
        // 将请求客户端的限流与期望并发对齐，避免内部信号量限制导致并发达不到预期
        let client = params.client.with_limit(concurrency);
        let downloader =
            Downloader::new_with_headers(client, DownloadConfig::default(), params.default_headers);
        let token = params.token_opt.unwrap_or_default();
        let total = params.urls.len() as i32;
        {
            let mut w = self.tasks.write();
            let t = w.entry(params.task_id.clone()).or_default();
            t.progress.total = total;
            t.status = TaskStatus::Running;
            t.updated_at = now_str();
        }
        let ct = token.clone();
        let tm = self.tasks.clone();
        tauri::async_runtime::spawn(async move {
            #[allow(clippy::useless_conversion)]
            let mut stream = stream::iter(params.urls.into_iter().zip(params.paths.into_iter()).map(|(u, p)| {
                let d = downloader.clone();
                let cancel = ct.clone();
                async move {
                    if cancel.is_cancelled() {
                        let res: anyhow::Result<()> = Err(anyhow::anyhow!("cancelled"));
                        return res;
                    }

                    d.download_file(&u, &p).await
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
                if let Some(t) = w.get_mut(&params.task_id) {
                    t.progress.current = current;
                    t.progress.total = total;
                    t.failed_count = failed_count;
                    t.updated_at = now_str();
                }
                if res.is_err() {
                    if let Some(t) = w.get_mut(&params.task_id) {
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
                if let Some(t) = w.get_mut(&params.task_id) {
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
                        TaskStatus::Queued => "queued".to_string(),
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
                    let _ = hm.set_dir_from_app(&params.app);
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
                    let _ = params.app.emit(
                        "download:completed",
                        serde_json::json!({"taskId": params.task_id , "taskName": tm.read().get(&params.task_id).unwrap().name}),
                    );
                }
                "cancelled" => {
                    let _ = params.app.emit(
                        "download:cancelled",
                        serde_json::json!({"taskId": params.task_id , "taskName": tm.read().get(&params.task_id).unwrap().name}),
                    );
                }
                _ => {
                    let _ = params.app.emit("download:failed", serde_json::json!({"taskId": params.task_id, "taskName": tm.read().get(&params.task_id).unwrap().name, "message": error_msg}));
                }
            }

            // 队列处理现在通过定期检查完成，无需手动触发
        });
        token
    }

    /// 增加重试计数
    pub fn increment_retry_count(&self, task_id: &str) {
        let mut w = self.tasks.write();
        if let Some(task) = w.get_mut(task_id) {
            task.retry_count += 1;
            task.last_retry_time = now_str();
            task.updated_at = now_str();
        }
    }

    /// 重置任务状态为完整重试（重新解析和下载）
    pub fn reset_for_full_retry(&self, task_id: &str) {
        let mut w = self.tasks.write();
        if let Some(task) = w.get_mut(task_id) {
            task.status = TaskStatus::Parsing;
            task.progress = Progress::default();
            task.failed_count = 0;
            task.error = String::new();
            task.updated_at = now_str();
        }
    }

    /// 重置失败文件以便重试（仅重试失败的文件）
    pub fn reset_failed_files_for_retry(&self, task_id: &str) -> Result<(), String> {
        let mut w = self.tasks.write();
        if let Some(task) = w.get_mut(task_id) {
            if task.status == TaskStatus::PartialFailed {
                // 重置进度，只重试失败的文件
                let success_count = task.progress.total - task.failed_count;
                task.progress.current = success_count;
                task.failed_count = 0;
                task.error = String::new();
                task.status = TaskStatus::Running;
                task.updated_at = now_str();
                Ok(())
            } else {
                Err("任务状态不是PartialFailed，无法重置失败文件".to_string())
            }
        } else {
            Err("任务不存在".to_string())
        }
    }

    /// 获取重试批量下载参数
    pub fn get_retry_batch_params(
        &self,
        task_id: &str,
        _client: RequestClient,
        _app: AppHandle,
    ) -> Option<BatchDownloadParams> {
        let _task = self.tasks.read().get(task_id)?.clone();

        // 这里需要根据实际情况重新构建下载参数
        // 由于我们需要原始的URL和路径信息，这部分可能需要从历史记录或其他地方获取
        // 为简化实现，这里返回None，实际使用时需要补充完整逻辑
        None
    }
}
