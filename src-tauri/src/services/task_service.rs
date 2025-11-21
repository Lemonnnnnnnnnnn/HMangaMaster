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

        // 预创建任务，便于前端尽早可见
        state.task_manager.read().create_or_start(&task_id, &url, 0);

        // 检查并发限制
        if state.task_manager.read().running_task_count() >= state.config.read().get_max_concurrent_tasks() {
            // 任务加入队列
            state.task_manager.read().create_or_start(&task_id, &url, 0);
            {
                let task_manager = state.task_manager.read();
                let mut w = task_manager.tasks.write();
                if let Some(t) = w.get_mut(&task_id) {
                    t.status = crate::task::TaskStatus::Queued;
                    t.updated_at = chrono::Utc::now().to_rfc3339();
                }
            }
            return Ok(task_id);
        }

        // 直接执行任务，不使用额外的异步spawn
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

}

impl Default for TaskService {
    fn default() -> Self {
        Self::new()
    }
}
