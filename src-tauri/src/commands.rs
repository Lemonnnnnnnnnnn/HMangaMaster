use tauri::State;

use crate::app::AppState;
use crate::history;
use crate::library;
use crate::logger;

// ---------- logger ----------
#[tauri::command]
pub fn logger_get_info(
    _state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<logger::LogInfo, String> {
    logger::get_log_info(&app).map_err(|e| e.to_string())
}

// ---------- config ----------
#[tauri::command]
pub fn config_get_active_library(state: State<AppState>) -> Result<String, String> {
    Ok(state.config.read().get_active_library())
}

#[tauri::command]
pub fn config_set_active_library(state: State<AppState>, library: String) -> Result<bool, String> {
    state
        .config
        .write()
        .set_active_library(library)
        .map(|_| true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn config_get_output_dir(state: State<AppState>) -> Result<String, String> {
    Ok(state.config.read().get_output_dir())
}

#[tauri::command]
pub async fn config_set_output_dir(
    state: State<'_, AppState>,
    window: tauri::Window,
) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(dir) = window.dialog().file().blocking_pick_folder() else {
        return Ok(false);
    };
    state
        .config
        .write()
        .set_output_dir(dir.to_string().to_string())
        .map(|_| true)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn config_get_proxy(state: State<AppState>) -> Result<String, String> {
    Ok(state.config.read().get_proxy())
}

#[tauri::command]
pub fn config_set_proxy(state: State<AppState>, proxy: String) -> Result<bool, String> {
    let mut w = state.config.write();
    w.set_proxy(proxy).map_err(|e| e.to_string())?;
    drop(w);
    // 代理更新后重建请求客户端
    state.rebuild_request_client().map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn config_get_libraries(state: State<AppState>) -> Result<Vec<String>, String> {
    Ok(state.config.read().get_libraries())
}

#[tauri::command]
pub async fn config_add_library(
    state: State<'_, AppState>,
    window: tauri::Window,
) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(dir) = window.dialog().file().blocking_pick_folder() else {
        return Ok(false);
    };
    state
        .config
        .write()
        .add_library(dir.to_string().to_string())
        .map(|_| true)
        .map_err(|e| e.to_string())
}

// ---------- library ----------
#[tauri::command]
pub fn library_init() -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub fn library_load(_state: State<AppState>, path: String) -> Result<bool, String> {
    // 前端会后续用 get_all_mangas 获取数据
    // 为对齐接口，这里直接返回 true
    Ok(!path.is_empty())
}

#[tauri::command]
pub fn library_load_active(state: State<AppState>) -> Result<bool, String> {
    let active = state.config.read().get_active_library();
    Ok(!active.is_empty())
}

#[tauri::command]
pub fn library_load_all() -> Result<bool, String> {
    Ok(true)
}

#[tauri::command]
pub fn library_get_all_mangas(state: State<AppState>) -> Result<Vec<library::Manga>, String> {
    let mgr = library::Manager::default();
    let libs = state.config.read().get_libraries();
    let mut all: Vec<library::Manga> = vec![];
    for lib in libs {
        if let Ok(mut v) = mgr.load_library(&lib) {
            all.append(&mut v);
        }
    }
    Ok(all)
}

#[tauri::command]
pub fn library_get_manga_images(
    _state: State<AppState>,
    path: String,
) -> Result<Vec<String>, String> {
    let mgr = library::Manager::default();
    mgr.get_manga_images(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_delete_manga(_state: State<AppState>, path: String) -> Result<bool, String> {
    let mgr = library::Manager::default();
    mgr.delete_manga(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn library_get_image_data_url(_state: State<AppState>, path: String) -> Result<String, String> {
    let mgr = library::Manager::default();
    mgr.get_image_data_url(&path).map_err(|e| e.to_string())
}

// ---------- history ----------
#[tauri::command]
pub fn history_get(
    _state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<history::DownloadTaskDTO>, String> {
    let mut mgr = history::Manager::default();
    let _ = mgr.set_dir_from_app(&app);
    Ok(mgr.get_history())
}

#[tauri::command]
pub fn history_add(
    _state: State<AppState>,
    app: tauri::AppHandle,
    record: history::DownloadTaskDTO,
) -> Result<(), String> {
    crate::services::HistoryService::add_history_record(&record, &app)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn history_clear(_state: State<AppState>, app: tauri::AppHandle) -> Result<(), String> {
    crate::services::HistoryService::clear_history(&app)
        .map_err(|e| e.to_string())
}

// ---------- task (最小下载批任务 + 取消) ----------
#[tauri::command]
pub fn task_all(state: State<AppState>) -> Result<Vec<crate::task::Task>, String> {
    Ok(state.task_service.get_all_tasks(&state))
}

#[tauri::command]
pub fn task_active(state: State<AppState>) -> Result<Vec<crate::task::Task>, String> {
    Ok(state.task_service.get_active_tasks(&state))
}

#[tauri::command]
pub fn task_by_id(
    state: State<AppState>,
    task_id: String,
) -> Result<Option<crate::task::Task>, String> {
    Ok(state.task_service.get_task_by_id(&task_id, &state))
}

#[tauri::command]
pub fn task_clear_history(state: State<AppState>) -> Result<bool, String> {
    state.task_service.clear_history_tasks(&state)
        .map_err(|e| e.to_string())
        .map(|_| true)
}

// 历史任务（从磁盘）
#[tauri::command]
pub fn task_history(
    _state: State<AppState>,
    app: tauri::AppHandle,
) -> Result<Vec<history::DownloadTaskDTO>, String> {
    crate::services::HistoryService::get_task_history(&app)
        .map_err(|e| e.to_string())
}

// 单任务进度
#[tauri::command]
pub fn task_progress(
    state: State<AppState>,
    task_id: String,
) -> Result<crate::task::Progress, String> {
    Ok(state.task_service.get_task_progress(&task_id, &state))
}

// ---------- crawler ----------
// 重构后的简化实现：使用TaskService处理所有复杂逻辑
#[tauri::command]
pub async fn task_start_crawl(
    state: State<'_, AppState>,
    app: tauri::AppHandle,
    url: String,
) -> Result<String, String> {
    state.task_service.start_crawl_task(url, app, &state).await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn task_cancel(
    state: State<AppState>,
    app: tauri::AppHandle,
    task_id: String,
) -> Result<bool, String> {
    state.task_service.cancel_task(&task_id, &app, &state)
        .map_err(|e| e.to_string())
}
