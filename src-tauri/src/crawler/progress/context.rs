//! 进度上下文管理
//!
//! 提供统一的进度报告和管理功能，封装了底层的进度报告器接口。

use crate::crawler::ProgressReporter;
use std::sync::Arc;

/// 进度管理上下文
///
/// 封装了进度报告的逻辑，提供统一的进度管理接口。
/// 支持前缀设置、进度更新、消息设置等功能。
pub struct ProgressContext {
    reporter: Option<Arc<dyn ProgressReporter>>,
    prefix: String,
}

impl Clone for ProgressContext {
    fn clone(&self) -> Self {
        Self {
            reporter: self.reporter.clone(),
            prefix: self.prefix.clone(),
        }
    }
}

impl ProgressContext {
    /// 创建新的进度上下文
    ///
    /// # 参数
    /// * `reporter` - 可选的进度报告器
    /// * `prefix` - 进度消息的前缀
    pub fn new(reporter: Option<Arc<dyn ProgressReporter>>, prefix: String) -> Self {
        Self { reporter, prefix }
    }

    /// 更新进度
    ///
    /// # 参数
    /// * `current` - 当前进度
    /// * `total` - 总进度
    /// * `message` - 进度消息
    pub fn update(&self, current: usize, total: usize, message: &str) {
        if let Some(r) = &self.reporter {
            r.set_task_name(&format!("{} - {} ({}/{})", self.prefix, message, current, total));
            r.set_total(total);
            if current > 0 {
                r.inc(1);
            }
        }
    }

    /// 设置消息
    ///
    /// # 参数
    /// * `message` - 要设置的消息
    pub fn set_message(&self, message: &str) {
        if let Some(r) = &self.reporter {
            r.set_task_name(&format!("{} - {}", self.prefix, message));
        }
    }

}
