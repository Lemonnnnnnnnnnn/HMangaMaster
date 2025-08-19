use crate::crawler::ProgressReporter;
use parking_lot::RwLock as PLRwLock;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use tauri::Emitter;

pub struct TaskParseReporter {
    id: String,
    app: tauri::AppHandle,
    total: AtomicUsize,
    current: AtomicUsize,
    task_mgr: Arc<PLRwLock<crate::task::TaskManager>>,
}

impl TaskParseReporter {
    pub fn new(id: String, app: tauri::AppHandle, task_mgr: Arc<PLRwLock<crate::task::TaskManager>>) -> Self {
        Self { id, app, total: AtomicUsize::new(0), current: AtomicUsize::new(0), task_mgr }
    }
}

impl ProgressReporter for TaskParseReporter {
    fn set_total(&self, total: usize) {
        self.total.store(total, Ordering::Relaxed);
        self.current.store(0, Ordering::Relaxed);
        let _ = self.app.emit("download:progress", serde_json::json!({
            "taskId": self.id,
            "type": "parsingTotal",
            "total": total
        }));
    }
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
    fn set_task_name(&self, name: &str) {
        self.task_mgr.read().set_name(&self.id, name);
        let _ = self.app.emit("download:progress", serde_json::json!({
            "taskId": self.id,
            "type": "taskName",
            "name": name
        }));
    }
}


