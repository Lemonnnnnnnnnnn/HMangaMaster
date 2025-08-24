//! 进度管理模块
//!
//! 提供统一的进度报告和管理功能

pub mod context;
pub mod reporters;

// 重新导出常用的类型
pub use context::ProgressContext;
pub use reporters::TaskReporter;

/// 进度报告器接口
///
/// 定义了通用的进度报告行为，支持设置总进度、增加进度和设置任务名称。
pub trait ProgressReporter: Send + Sync {
    /// 设置总进度
    fn set_total(&self, _total: usize) {}
    /// 增加进度
    fn inc(&self, _delta: usize) {}
    /// 设置任务名称
    fn set_task_name(&self, _name: &str) {}
}
