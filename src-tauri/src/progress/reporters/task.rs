//! 任务进度报告器实现
//!
//! 提供基于Tauri的事件驱动的任务进度报告器实现。

use crate::progress::ProgressReporter;
use parking_lot::RwLock as PLRwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

/// 基于Tauri的任务解析进度报告器
///
/// 通过Tauri的事件系统向前端报告解析进度。
pub struct TaskReporter {
    id: String,
    total: AtomicUsize,
    current: AtomicUsize,
    task_mgr: Arc<PLRwLock<crate::task::TaskManager>>,
}

impl TaskReporter {
    /// 创建新的任务进度报告器
    ///
    /// # 参数
    /// * `id` - 任务ID
    /// * `app` - Tauri应用句柄
    /// * `task_mgr` - 任务管理器
    pub fn new(id: String, task_mgr: Arc<PLRwLock<crate::task::TaskManager>>) -> Self {
        Self {
            id,
            total: AtomicUsize::new(0),
            current: AtomicUsize::new(0),
            task_mgr,
        }
    }
}

impl ProgressReporter for TaskReporter {
    /// 设置总进度
    fn set_total(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.current.store(0, Ordering::Relaxed);
    }

    /// 增加进度
    fn inc(&self, delta: usize) {
        let _ = self.current.fetch_add(delta, Ordering::Relaxed);
    }

    /// 设置任务名称
    fn set_task_name(&self, name: &str) {
        self.task_mgr.read().set_name(&self.id, name);
    }
}
