use tauri::Manager;
mod app;
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
mod services;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(app::AppState::new())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let app_handle = app.handle();
            let state = app.state::<app::AppState>();
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
            // crawler
            commands::task_start_crawl,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
