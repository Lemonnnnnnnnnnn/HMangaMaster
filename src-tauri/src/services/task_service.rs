use tauri::{AppHandle, Emitter};
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::AppState;
use crate::config::service::ConfigService;
use crate::services::{CrawlService};

/// 任务服务错误类型
#[derive(Debug)]
pub enum TaskError {
    CrawlError(String),
    HistoryError(String),
}

impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TaskError::CrawlError(msg) => write!(f, "爬虫错误: {}", msg),
            TaskError::HistoryError(msg) => write!(f, "历史记录错误: {}", msg),
        }
    }
}

impl std::error::Error for TaskError {}

/// 任务服务
pub struct TaskService;

impl TaskService {
    pub fn new() -> Self {
        Self
    }

    /// 启动爬虫任务
    pub async fn start_crawl_task(
        &self,
        url: String,
        app: AppHandle,
        state: &AppState,
    ) -> Result<String, TaskError> {
        // 生成任务ID
        let task_id = Uuid::new_v4().to_string();

        // 检查并发限制
        if state.task_manager.read().running_task_count() >= state.config.read().get_max_concurrent_tasks() {
            // 任务加入队列 - 直接创建为Queued状态
            {
                let task_manager = state.task_manager.read();
                let mut w = task_manager.tasks.write();
                let mut t = w.remove(&task_id).unwrap_or_default();
                t.id = task_id.clone();
                t.url = url.clone();
                t.status = crate::task::TaskStatus::Queued;
                t.progress = crate::task::Progress { current: 0, total: 0 };
                t.start_time = chrono::Utc::now().to_rfc3339();
                t.updated_at = t.start_time.clone();
                w.insert(task_id.clone(), t);
            }
            return Ok(task_id);
        }

        // 直接执行任务，先创建为Parsing状态
        state.task_manager.read().create_or_start(&task_id, &url, 0);
        Self::execute_crawl_task_internal(&task_id, &url, &app, state).await?;

        Ok(task_id)
    }

    /// 内部任务执行逻辑
    async fn execute_crawl_task_internal(
        task_id: &str,
        url: &str,
        app: &AppHandle,
        state: &AppState,
    ) -> Result<(), TaskError> {
        // 获取必要配置
        let client = state.request.read().clone();
        let output_dir = state.config.read().get_output_dir();

        // 创建取消令牌
        let cancel_token = CancellationToken::new();
        state
            .cancels
            .write()
            .insert(task_id.to_string(), cancel_token.clone());

        // 解析URL
        let parsed = match CrawlService::parse_and_validate(
            &client,
            url,
            task_id,
            &state.task_manager,
            &cancel_token,
            Some(state),
        ).await {
            Ok(p) => p,
            Err(e) => {
                // 处理解析错误 - 简化版本直接设置失败状态
                state.task_manager.read().set_failed(task_id, &e.to_string());
                let _ = app.emit("download:failed", serde_json::json!({"taskId": task_id, "message": e.to_string()}));
                return Err(TaskError::CrawlError(e.to_string()));
            }
        };

        // 构建下载计划
        let (urls, paths) = CrawlService::build_download_plan(&parsed, &output_dir);
        let (name, save_path) = CrawlService::prepare_task_info(&parsed, &output_dir);

        // 更新任务信息并切换到下载状态
        state.task_manager.read().set_name_and_path(task_id, &name, &save_path);
        state.task_manager.read().set_status_downloading(task_id, urls.len() as i32);

        // 启动批量下载，使用推荐并发数或默认值
        let batch_params = crate::task::manager::BatchDownloadParams {
            app: app.clone(),
            task_id: task_id.to_string(),
            urls,
            paths,
            client,
            token_opt: Some(cancel_token.clone()),
            default_headers: parsed.download_headers,
            concurrency_override: parsed.recommended_concurrency,
        };
        let token = state.task_manager.read().start_batch_with_concurrency(batch_params);

        // 更新取消令牌
        state.cancels.write().insert(task_id.to_string(), token);

        Ok(())
    }

    /// 处理排队中的任务
    pub async fn process_queued_tasks(
        &self,
        app: &AppHandle,
        state: &AppState,
    ) -> Result<(), TaskError> {
        // 检查是否有可用容量
        while state.task_manager.read().running_task_count() < state.config.read().get_max_concurrent_tasks() {
            // 获取下一个排队中的任务
            let queued_task = state.task_manager.read().get_next_queued_task();

            if let Some(task) = queued_task {
                let task_id = task.id.clone();
                let url = task.url.clone();

                // 将任务从Queued状态转换为Parsing状态
                if state.task_manager.read().start_queued_task(&task_id) {
                    // 执行任务
                    match Self::execute_crawl_task_internal(&task_id, &url, app, state).await {
                        Ok(()) => {
                            // 任务执行成功，继续处理下一个
                        }
                        Err(e) => {
                            // 任务执行失败，记录错误但继续处理其他排队任务
                            eprintln!("Failed to execute queued task {}: {}", task_id, e);
                        }
                    }
                } else {
                    break; // 无法启动任务，可能已被其他处理者占用
                }
            } else {
                break; // 没有更多排队任务
            }
        }

        Ok(())
    }

    /// 取消任务
    pub fn cancel_task(
        &self,
        task_id: &str,
        app: &AppHandle,
        state: &AppState,
    ) -> Result<bool, TaskError> {
        if let Some(token) = state.cancels.write().remove(task_id) {
            token.cancel();
            state.task_manager.read().set_cancelled(task_id);

            let _ = app.emit("download:cancelled", serde_json::json!({"taskId": task_id}));
            // 队列处理现在通过定期检查完成，无需手动触发
            Ok(true)
        } else {
            Ok(false)
        }
    }

    /// 获取所有任务
    pub fn get_all_tasks(&self, state: &AppState) -> Vec<crate::task::Task> {
        state.task_manager.read().all()
    }

    /// 获取活跃任务
    pub fn get_active_tasks(&self, state: &AppState) -> Vec<crate::task::Task> {
        state.task_manager.read().active()
    }

    /// 根据ID获取任务
    pub fn get_task_by_id(&self, task_id: &str, state: &AppState) -> Option<crate::task::Task> {
        state.task_manager.read().by_id(task_id)
    }

    /// 获取任务进度
    pub fn get_task_progress(&self, task_id: &str, state: &AppState) -> crate::task::Progress {
        state.task_manager.read()
            .by_id(task_id)
            .map(|t| t.progress.clone())
            .unwrap_or_default()
    }

    /// 清空历史任务
    pub fn clear_history_tasks(&self, state: &AppState) -> Result<(), TaskError> {
        state.task_manager.read().clear_non_active();
        Ok(())
    }

    /// 完整重试失败的任务（重新解析和下载）
    pub async fn retry_task(
        &self,
        task_id: &str,
        app: &AppHandle,
        state: &AppState,
    ) -> Result<(), TaskError> {
        // 检查任务是否可重试
        let task = state.task_manager.read().by_id(task_id)
            .ok_or_else(|| TaskError::CrawlError("任务不存在".to_string()))?;

        if !self.is_task_retryable(&task) {
            return Err(TaskError::CrawlError("任务不可重试或已达到最大重试次数".to_string()));
        }

        // 增加重试计数
        state.task_manager.read().increment_retry_count(task_id);

        // 重置任务状态为解析中
        state.task_manager.read().reset_for_full_retry(task_id);

        // 重新执行任务
        Self::execute_crawl_task_internal(task_id, &task.url, app, state).await?;

        Ok(())
    }

    /// 部分重试失败的任务（仅重试失败的文件）
    // TODO: 需要存储原始URL和路径信息以实现此功能
    pub async fn retry_failed_files_only(
        &self,
        _task_id: &str,
        _app: &AppHandle,
        _state: &AppState,
    ) -> Result<(), TaskError> {
        // 暂时返回错误，提示使用完整重试
        Err(TaskError::CrawlError(
            "重试失败文件功能暂未实现，建议使用「完整重试」重新下载所有文件".to_string()
        ))
    }

    /// 检查任务是否可重试
    pub fn is_task_retryable(&self, task: &crate::task::Task) -> bool {
        match task.status {
            crate::task::TaskStatus::Failed | crate::task::TaskStatus::PartialFailed => {
                task.retryable && task.retry_count < task.max_retries
            }
            _ => false,
        }
    }

}

impl Default for TaskService {
    fn default() -> Self {
        Self::new()
    }
}
