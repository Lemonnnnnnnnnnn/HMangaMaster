use tauri::State;
use tauri::Emitter;

use crate::app::AppState;
// use crate::config::Manager as ConfigManager;
use crate::logger;
use crate::library;
use crate::history;
use crate::download;
use crate::crawler;
// use tokio_util::sync::CancellationToken;
// use crate::task::TaskManager;

// ---------- logger ----------
#[tauri::command]
pub fn logger_get_info(_state: State<AppState>, app: tauri::AppHandle) -> Result<logger::LogInfo, String> {
    logger::get_log_info(&app).map_err(|e| e.to_string())
}

// ---------- config ----------
#[tauri::command]
pub fn config_get_active_library(state: State<AppState>) -> Result<String, String> {
    Ok(state.config.read().get_active_library())
}

#[tauri::command]
pub fn config_set_active_library(state: State<AppState>, library: String) -> Result<bool, String> {
    state.config.write().set_active_library(library).map(|_| true).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn config_get_output_dir(state: State<AppState>) -> Result<String, String> {
    Ok(state.config.read().get_output_dir())
}

#[tauri::command]
pub async fn config_set_output_dir(state: State<'_, AppState>, window: tauri::Window) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(dir) = window.dialog().file().blocking_pick_folder() else { return Ok(false); };
    state.config.write().set_output_dir(dir.to_string().to_string()).map(|_| true).map_err(|e| e.to_string())
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
pub async fn config_add_library(state: State<'_, AppState>, window: tauri::Window) -> Result<bool, String> {
    use tauri_plugin_dialog::DialogExt;
    let Some(dir) = window.dialog().file().blocking_pick_folder() else { return Ok(false); };
    state.config.write().add_library(dir.to_string().to_string()).map(|_| true).map_err(|e| e.to_string())
}

// ---------- library ----------
#[tauri::command]
pub fn library_init() -> Result<bool, String> { Ok(true) }

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
pub fn library_load_all() -> Result<bool, String> { Ok(true) }

#[tauri::command]
pub fn library_get_all_mangas(state: State<AppState>) -> Result<Vec<library::Manga>, String> {
    let mgr = library::Manager::default();
    let libs = state.config.read().get_libraries();
    let mut all: Vec<library::Manga> = vec![];
    for lib in libs { if let Ok(mut v) = mgr.load_library(&lib) { all.append(&mut v); } }
    Ok(all)
}

#[tauri::command]
pub fn library_get_manga_images(_state: State<AppState>, path: String) -> Result<Vec<String>, String> {
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
pub fn history_get(_state: State<AppState>, app: tauri::AppHandle) -> Result<Vec<history::DownloadTaskDTO>, String> {
    let mut mgr = history::Manager::default();
    let _ = mgr.set_dir_from_app(&app);
    Ok(mgr.get_history())
}

#[tauri::command]
pub fn history_add(_state: State<AppState>, app: tauri::AppHandle, record: history::DownloadTaskDTO) -> Result<(), String> {
    let mut mgr = history::Manager::default();
    let _ = mgr.set_dir_from_app(&app);
    mgr.add_record(record);
    Ok(())
}

#[tauri::command]
pub fn history_clear(_state: State<AppState>, app: tauri::AppHandle) -> Result<(), String> {
    let mut mgr = history::Manager::default();
    let _ = mgr.set_dir_from_app(&app);
    mgr.clear();
    Ok(())
}

// ---------- download ----------
#[tauri::command]
pub async fn download_file(state: State<'_, AppState>, app: tauri::AppHandle, url: String, save_path: String) -> Result<bool, String> {
    use std::path::PathBuf;
    let client = { state.request.read().clone() };
    let downloader = download::Downloader::new(client, download::Config::default());
    let path = PathBuf::from(save_path);
    match downloader.download_file(&url, &path).await {
        Ok(_) => {
            let _ = app.emit("download:completed", &url);
            Ok(true)
        }
        Err(e) => {
            let _ = app.emit("download:failed", serde_json::json!({"url": url, "message": e.to_string()}));
            Err(e.to_string())
        }
    }
}

// ---------- task (最小下载批任务 + 取消) ----------
#[tauri::command]
pub fn task_all(state: State<AppState>) -> Result<Vec<crate::task::Task>, String> {
    Ok(state.task_manager.read().all())
}

#[tauri::command]
pub fn task_active(state: State<AppState>) -> Result<Vec<crate::task::Task>, String> {
    Ok(state.task_manager.read().active())
}

#[tauri::command]
pub fn task_by_id(state: State<AppState>, task_id: String) -> Result<Option<crate::task::Task>, String> {
    Ok(state.task_manager.read().by_id(&task_id))
}

#[tauri::command]
pub fn task_clear_history(state: State<AppState>) -> Result<bool, String> {
    state.task_manager.read().clear_non_active();
    Ok(true)
}

// 历史任务（从磁盘）
#[tauri::command]
pub fn task_history(_state: State<AppState>, app: tauri::AppHandle) -> Result<Vec<history::DownloadTaskDTO>, String> {
    let mut mgr = history::Manager::default();
    let _ = mgr.set_dir_from_app(&app);
    Ok(mgr.get_history())
}

// 单任务进度
#[tauri::command]
pub fn task_progress(state: State<AppState>, task_id: String) -> Result<crate::task::Progress, String> {
    Ok(state
        .task_manager
        .read()
        .by_id(&task_id)
        .map(|t| t.progress.clone())
        .unwrap_or_default())
}

// ---------- crawler ----------
// 最小实现：创建一个解析任务，解析出若干图片 URL，然后复用批量下载逻辑
#[tauri::command]
pub async fn task_start_crawl(state: State<'_, AppState>, app: tauri::AppHandle, url: String) -> Result<String, String> {
    // 生成 task_id
    let task_id = uuid::Uuid::new_v4().to_string();
    // 在解析前就创建任务，便于前端尽早可见（解析阶段暂不计入进度）
    state.task_manager.read().create_or_start(&task_id, &url, 0);
    // 解析（同步/异步）
    let client = { state.request.read().clone() };
    let output_dir = state.config.read().get_output_dir();
    // 为任务创建取消令牌并注册，解析与下载公用
    let cancel_token = tokio_util::sync::CancellationToken::new();
    state.cancels.write().insert(task_id.clone(), cancel_token.clone());
    // 解析阶段进度上报适配：仅设置 stage/total，不改变下载进度条
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};
    struct TaskParseReporter {
        id: String,
        app: tauri::AppHandle,
        total: AtomicUsize,
        current: AtomicUsize,
    }
    impl crawler::ProgressReporter for TaskParseReporter {
        fn set_total(&self, total: usize) {
            self.total.store(total, Ordering::Relaxed);
            self.current.store(0, Ordering::Relaxed);
            let _ = self.app.emit("download:progress", serde_json::json!({
                "taskId": self.id,
                "stage": "parsingTotal",
                "total": total
            }));
        }
        fn inc(&self, delta: usize) {
            let new_cur = self.current.fetch_add(delta, Ordering::Relaxed) + delta;
            let total = self.total.load(Ordering::Relaxed);
            let _ = self.app.emit("download:progress", serde_json::json!({
                "taskId": self.id,
                "stage": "parsingProgress",
                "current": new_cur,
                "total": total
            }));
        }
        fn set_stage(&self, stage: &str) { let _ = self.app.emit("download:progress", serde_json::json!({"taskId": self.id, "stage": stage})); }
    }
    let reporter = Arc::new(TaskParseReporter { id: task_id.clone(), app: app.clone(), total: AtomicUsize::new(0), current: AtomicUsize::new(0) });
    // 解析阶段支持取消
    let parsed = match {
        let fut = crawler::parse_gallery_auto(&client, &url, Some(reporter));
        tokio::select! {
            biased;
            _ = cancel_token.cancelled() => Err(anyhow::anyhow!("cancelled")),
            res = fut => res,
        }
    } {
        Ok(p) => p,
        Err(e) => {
            // 若为取消，标记为取消；否则标记失败
            if cancel_token.is_cancelled() {
                state.task_manager.read().set_cancelled(&task_id);
            } else {
                state.task_manager.read().set_failed(&task_id, &e.to_string());
            }
            // 写入历史
            {
                let t = state.task_manager.read().by_id(&task_id);
                if let Some(t) = t {
                    let mut hm = history::Manager::default();
                    let _ = hm.set_dir_from_app(&app);
                    let dto = history::DownloadTaskDTO {
                        id: t.id.clone(), url: t.url.clone(),
                        status: match t.status { crate::task::TaskStatus::Cancelled => "cancelled".into(), crate::task::TaskStatus::Failed => "failed".into(), _ => "failed".into() },
                        save_path: t.save_path.clone(), start_time: t.start_time.clone(), complete_time: t.complete_time.clone(), updated_at: t.updated_at.clone(), error: t.error.clone(), name: t.name.clone(), progress: history::Progress { current: t.progress.current, total: t.progress.total }
                    };
                    hm.add_record(dto);
                }
            }
            if cancel_token.is_cancelled() {
                let _ = app.emit("download:cancelled", serde_json::json!({"taskId": task_id}));
            } else {
                let _ = app.emit("download:failed", serde_json::json!({"taskId": task_id, "message": e.to_string()}));
            }
            return Err(e.to_string());
        }
    };
    if parsed.image_urls.is_empty() { return Err("未解析到图片".into()); }
    // 生成保存路径
    let safe_name = sanitize_filename::sanitize(parsed.title.unwrap_or_else(|| "gallery".to_string()));
    let base_path = std::path::PathBuf::from(output_dir).join(&safe_name);
    let mut urls: Vec<String> = vec![];
    let mut paths: Vec<std::path::PathBuf> = vec![];
    for (idx, u) in parsed.image_urls.iter().enumerate() {
        urls.push(u.clone());
        let ext = infer_ext_from_url(u).unwrap_or("jpg");
        let filename = format!("{:04}.{}", idx + 1, ext);
        paths.push(base_path.join(&filename));
    }
    // 解析结束后，设置任务可见的名称与保存目录，并更新总数为图片数量；切换为 downloading
    state.task_manager.read().set_name_and_path(&task_id, &safe_name, base_path.to_string_lossy().as_ref());
    state.task_manager.read().set_status_downloading(&task_id, urls.len() as i32);
    // 启动批量下载任务（按站点注入必要请求头，例如 Hitomi 需要 Referer）
    let headers_opt = if url.contains("hitomi.la") {
        let mut h = reqwest::header::HeaderMap::new();
        h.insert(reqwest::header::REFERER, reqwest::header::HeaderValue::from_static("https://hitomi.la/"));
        Some(h)
    } else { None };
    let token = state.task_manager.read().start_batch(app.clone(), task_id.clone(), urls, paths, client, Some(cancel_token.clone()), headers_opt);
    state.cancels.write().insert(task_id.clone(), token);
    Ok(task_id)
}

fn infer_ext_from_url(url: &str) -> Option<&'static str> {
    let path = reqwest::Url::parse(url).ok()?.path().to_ascii_lowercase();
    if path.ends_with(".webp") { return Some("webp"); }
    if path.ends_with(".jpg") || path.ends_with(".jpeg") { return Some("jpg"); }
    if path.ends_with(".png") { return Some("png"); }
    if path.ends_with(".gif") { return Some("gif"); }
    None
}
#[tauri::command]
pub async fn task_start_batch_download(state: State<'_, AppState>, app: tauri::AppHandle, task_id: String, urls: Vec<String>, save_paths: Vec<String>) -> Result<bool, String> {
    use std::path::PathBuf;
    if urls.len() != save_paths.len() { return Err("urls 与 save_paths 数量不一致".into()); }
    let paths: Vec<PathBuf> = save_paths.into_iter().map(PathBuf::from).collect();
    let client = { state.request.read().clone() };
    let token = state.task_manager.read().start_batch(app, task_id.clone(), urls, paths, client, None, None);
    state.cancels.write().insert(task_id.clone(), token);
    Ok(true)
}

#[tauri::command]
pub fn task_cancel(state: State<AppState>, app: tauri::AppHandle, task_id: String) -> Result<bool, String> {
    if let Some(token) = state.cancels.write().remove(&task_id) {
        token.cancel();
        state.task_manager.read().set_cancelled(&task_id);
        let _ = app.emit("download:cancelled", serde_json::json!({"taskId": task_id}));
        return Ok(true)
    }
    Ok(false)
}


