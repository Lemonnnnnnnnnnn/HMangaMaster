//! 进度报告器实现
//!
//! 提供基于Tauri的事件驱动的进度报告器实现。

use crate::crawler::ProgressReporter;
use parking_lot::RwLock as PLRwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::Emitter;

/// 基于Tauri的任务解析进度报告器
///
/// 通过Tauri的事件系统向前端报告解析进度。
pub struct TaskParseReporter {
    id: String,
    app: tauri::AppHandle,
    total: AtomicUsize,
    current: AtomicUsize,
    task_mgr: Arc<PLRwLock<crate::task::TaskManager>>,
}

impl TaskParseReporter {
    /// 创建新的任务解析进度报告器
    ///
    /// # 参数
    /// * `id` - 任务ID
    /// * `app` - Tauri应用句柄
    /// * `task_mgr` - 任务管理器
    pub fn new(id: String, app: tauri::AppHandle, task_mgr: Arc<PLRwLock<crate::task::TaskManager>>) -> Self {
        Self { id, app, total: AtomicUsize::new(0), current: AtomicUsize::new(0), task_mgr }
    }
}

impl ProgressReporter for TaskParseReporter {
    /// 设置总进度
    fn set_total(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.current.store(0, Ordering::Relaxed);
        let _ = self.app.emit("download:progress", serde_json::json!({
            "taskId": self.id,
            "type": "parsingTotal",
            "total": total
        }));
    }

    /// 增加进度
    fn inc(&self, delta: usize) {
        let new_cur = self.current.fetch_add(delta, Ordering::Relaxed) + delta;
        let total = self.total.load(Ordering::Relaxed);
        let _ = self.app.emit("download:progress", serde_json::json!({
            "taskId": self.id,
            "type": "parsingProgress",
            "current": new_cur,
            "total": total
        }));
    }

    /// 设置任务名称
    fn set_task_name(&self, name: &str) {
        self.task_mgr.read().set_name(&self.id, name);
        let _ = self.app.emit("download:progress", serde_json::json!({
            "taskId": self.id,
            "type": "taskName",
            "name": name
        }));
    }
}
