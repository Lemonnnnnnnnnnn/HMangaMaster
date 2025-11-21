use tauri::{AppHandle, Emitter};

use crate::AppState;
use crate::batch_crawler;

/// 批量服务错误类型
#[derive(Debug)]
pub enum BatchError {
    CrawlError(String),
    TaskError(String),
}

impl std::fmt::Display for BatchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BatchError::CrawlError(msg) => write!(f, "批量解析错误: {}", msg),
            BatchError::TaskError(msg) => write!(f, "任务创建错误: {}", msg),
        }
    }
}

impl std::error::Error for BatchError {}

/// 批量服务
pub struct BatchService;

impl BatchService {
    pub fn new() -> Self {
        Self
    }

    /// 启动批量爬虫任务
    pub async fn start_batch_crawl(
        &self,
        url: String,
        app: AppHandle,
        state: &AppState,
    ) -> Result<Vec<String>, BatchError> {
        // 获取必要配置
        let client = state.request.read().clone();

        // 1. 提取所有漫画链接
        let manga_links = batch_crawler::extract_manga_links_auto(
            &client,
            &url,
            None,
            Some(state),
        ).await.map_err(|e| BatchError::CrawlError(e.to_string()))?;

        if manga_links.is_empty() {
            return Err(BatchError::CrawlError("未找到任何漫画链接".to_string()));
        }

        // 发送批量解析完成事件
        let _ = app.emit("batch:extracted", serde_json::json!({
            "url": url,
            "count": manga_links.len()
        }));

        // 2. 为每个漫画链接创建下载任务
        let mut task_ids = Vec::new();

        for manga_url in manga_links {
            // 创建任务
            let task_id = state.task_service.start_crawl_task(
                manga_url.clone(),
                app.clone(),
                state,
            ).await.map_err(|e| BatchError::TaskError(e.to_string()))?;

            task_ids.push(task_id);

            // 短暂延迟，避免同时启动过多任务
            tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
        }

        // 发送批量任务创建完成事件
        let _ = app.emit("batch:started", serde_json::json!({
            "sourceUrl": url,
            "taskIds": task_ids,
            "totalTasks": task_ids.len()
        }));

        Ok(task_ids)
    }
}

impl Default for BatchService {
    fn default() -> Self {
        Self::new()
    }
}