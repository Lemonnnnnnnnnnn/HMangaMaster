use std::sync::Arc;
use parking_lot::RwLock;
use tauri::AppHandle;

use crate::config::Manager as ConfigManager;
use crate::logger::Logger;
use crate::request::Client as RequestClient;
use crate::services::TaskService;
use tokio_util::sync::CancellationToken;
use std::collections::HashMap;
use crate::task::TaskManager;
// use crate::library::Manager as LibraryManager;
// use crate::history::Manager as HistoryManager;

pub struct AppState {
    pub logger: Arc<Logger>,
    pub config: Arc<RwLock<ConfigManager>>,
    pub request: Arc<RwLock<RequestClient>>,
    pub cancels: Arc<RwLock<HashMap<String, CancellationToken>>>,
    pub task_manager: Arc<RwLock<TaskManager>>,
    pub task_service: Arc<TaskService>,
    // 预留：后续若需要共享实例再接入
    // pub library: Arc<RwLock<LibraryManager>>,
    // pub history: Arc<RwLock<HistoryManager>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            logger: Arc::new(Logger::new()),
            config: Arc::new(RwLock::new(ConfigManager::default())),
            request: Arc::new(RwLock::new(RequestClient::new(None).unwrap())),
            cancels: Arc::new(RwLock::new(HashMap::new())),
            task_manager: Arc::new(RwLock::new(TaskManager::default())),
            task_service: Arc::new(TaskService::new()),
        }
    }

    pub fn init_logger(&self, handle: AppHandle) -> anyhow::Result<()> {
        self.logger.init(&handle)?;
        Ok(())
    }

    pub fn init_config(&self, handle: AppHandle) -> anyhow::Result<()> {
        {
            let mut mgr = self.config.write();
            mgr.set_path_from_app(&handle)?;
            mgr.load_or_default()?;
        }
        // 初始化请求客户端与配置的代理对齐（需在释放写锁后调用，避免死锁）
        self.rebuild_request_client()?;
        Ok(())
    }

    pub fn rebuild_request_client(&self) -> anyhow::Result<()> {
        let proxy = self.config.read().get_proxy();
        let proxy_opt = if proxy.is_empty() { None } else { Some(proxy) };
        let client = RequestClient::new(proxy_opt)?;
        *self.request.write() = client;
        Ok(())
    }
}


