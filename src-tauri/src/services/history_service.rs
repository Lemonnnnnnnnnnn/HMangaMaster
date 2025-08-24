use tauri::AppHandle;

use crate::history;
use crate::task::{Task, TaskStatus};

/// 历史记录服务错误类型
#[derive(Debug)]
pub enum HistoryError {
    IoError(String),
}

impl std::fmt::Display for HistoryError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HistoryError::IoError(msg) => write!(f, "IO错误: {}", msg),
        }
    }
}

impl std::error::Error for HistoryError {}

/// 历史记录服务
pub struct HistoryService;

impl HistoryService {
    /// 记录任务结果到历史记录
    pub fn record_task_result(
        task: &Task,
        app: &AppHandle,
    ) -> Result<(), HistoryError> {
        let mut hm = history::Manager::default();
        let _ = hm.set_dir_from_app(app);

        let dto = history::DownloadTaskDTO {
            id: task.id.clone(),
            url: task.url.clone(),
            status: Self::map_task_status_to_string(&task.status),
            save_path: task.save_path.clone(),
            start_time: task.start_time.clone(),
            complete_time: task.complete_time.clone(),
            updated_at: task.updated_at.clone(),
            error: task.error.clone(),
            failed_count: task.failed_count,
            name: task.name.clone(),
            progress: history::Progress {
                current: task.progress.current,
                total: task.progress.total,
            },
        };

        hm.add_record(dto);
        Ok(())
    }



    /// 将任务状态转换为字符串
    fn map_task_status_to_string(status: &TaskStatus) -> String {
        match status {
            TaskStatus::Pending => "pending".to_string(),
            TaskStatus::Parsing => "parsing".to_string(),
            TaskStatus::Running => "downloading".to_string(),
            TaskStatus::Completed => "completed".to_string(),
            TaskStatus::PartialFailed => "partial_failed".to_string(),
            TaskStatus::Failed => "failed".to_string(),
            TaskStatus::Cancelled => "cancelled".to_string(),
        }
    }

    /// 创建历史记录管理器
    pub fn create_manager(app: &AppHandle) -> Result<history::Manager, HistoryError> {
        let mut hm = history::Manager::default();
        hm.set_dir_from_app(app)
            .map_err(|e| HistoryError::IoError(e.to_string()))?;
        Ok(hm)
    }



    /// 清空历史记录
    pub fn clear_history(app: &AppHandle) -> Result<(), HistoryError> {
        let mut hm = Self::create_manager(app)?;
        hm.clear();
        Ok(())
    }

    /// 添加历史记录
    pub fn add_history_record(
        record: &history::DownloadTaskDTO,
        app: &AppHandle,
    ) -> Result<(), HistoryError> {
        let mut hm = Self::create_manager(app)?;
        hm.add_record(record.clone());
        Ok(())
    }

    /// 获取任务历史记录（从磁盘）
    pub fn get_task_history(app: &AppHandle) -> Result<Vec<history::DownloadTaskDTO>, HistoryError> {
        let hm = Self::create_manager(app)?;
        Ok(hm.get_history())
    }
}
