use tauri::AppHandle;

use crate::history;

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
