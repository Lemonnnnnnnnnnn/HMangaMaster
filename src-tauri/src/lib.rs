use std::sync::Arc;
use parking_lot::RwLock;
use tauri::{AppHandle, Manager};
use tokio_util::sync::CancellationToken;
use std::collections::HashMap;

use crate::config::service::{AppConfigService, ConfigService};
use crate::logger::Logger;
use crate::request::RequestClient;
use crate::task::TaskManager;
use crate::services::TaskService;

mod commands;
mod logger;
mod config;
mod library;
mod history;
mod request;
mod download;
mod task;
mod crawler;
mod progress;
mod batch_crawler;
mod services;

#[derive(Clone)]
pub struct AppState {
    pub logger: Arc<Logger>,
    pub config: Arc<RwLock<AppConfigService>>,
    pub request: Arc<RwLock<RequestClient>>,
    pub cancels: Arc<RwLock<HashMap<String, CancellationToken>>>,
    pub task_manager: Arc<RwLock<TaskManager>>,
    pub task_service: Arc<TaskService>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            logger: Arc::new(Logger::default()),
            config: Arc::new(RwLock::new(AppConfigService::default())),
            request: Arc::new(RwLock::new(RequestClient::new(None).unwrap())),
            cancels: Arc::new(RwLock::new(HashMap::new())),
            task_manager: Arc::new(RwLock::new(TaskManager::default())),
            task_service: Arc::new(TaskService::new()),
        }
    }
}

impl AppState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn init_logger(&self, handle: AppHandle) -> anyhow::Result<()> {
        self.logger.init(&handle)?;
        Ok(())
    }

    pub fn init_config(&self, handle: AppHandle) -> anyhow::Result<()> {
        let max_concurrent_tasks = {
            let mut config = self.config.write();
            config.set_path_from_app(&handle)?;
            config.load_or_default()?;

            // 获取并发限制配置
            config.get_max_concurrent_tasks()
        };

        // 初始化任务管理器的并发限制
        self.task_manager.write().set_max_concurrent_tasks(max_concurrent_tasks);

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

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle();
            let state = app.state::<AppState>();
            state.init_logger(app_handle.clone())?;
            state.init_config(app_handle.clone())?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            // config
            commands::config_get_active_library,
            commands::config_set_active_library,
            commands::config_get_output_dir,
            commands::config_set_output_dir,
            commands::config_get_proxy,
            commands::config_set_proxy,
            commands::config_get_libraries,
            commands::config_add_library,
            commands::config_get_parser_config,
            commands::config_set_parser_config,
            commands::config_get_all_parser_configs,
            commands::config_get_config_path,
            commands::config_get_max_concurrent_tasks,
            commands::config_set_max_concurrent_tasks,
            // logger
            commands::logger_get_info,
            // library
            commands::library_init,
            commands::library_load,
            commands::library_load_active,
            commands::library_load_all,
            commands::library_get_all_mangas,
            commands::library_get_manga_images,
            commands::library_delete_manga,
            // history
            commands::history_get,
            commands::history_add,
            commands::history_clear,
            // task
            commands::task_cancel,
            commands::task_all,
            commands::task_active,
            commands::task_by_id,
            commands::task_clear_history,
            commands::task_history,
            commands::task_progress,
            commands::task_get_status,
            // crawler
            commands::task_start_crawl,
            // batch
            commands::batch_start_crawl,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
