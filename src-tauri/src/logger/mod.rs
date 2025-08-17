use std::path::PathBuf;
use tracing::info;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{fmt, layer::SubscriberExt, EnvFilter, Registry};
use tauri::Manager;

pub struct Logger {
    inited: std::sync::atomic::AtomicBool,
}

impl Logger {
    pub fn new() -> Self {
        Self { inited: std::sync::atomic::AtomicBool::new(false) }
    }

    pub fn init(&self, app: &tauri::AppHandle) -> anyhow::Result<()> {
        use std::sync::atomic::Ordering;
        if self.inited.swap(true, Ordering::SeqCst) {
            return Ok(());
        }

        let log_dir = default_log_dir(app)?;
        std::fs::create_dir_all(&log_dir)?;
        let file_appender = RollingFileAppender::new(Rotation::DAILY, &log_dir, "app.log");
        let (non_blocking, _guard) = tracing_appender::non_blocking(file_appender);

        let env_filter = EnvFilter::try_from_default_env()
            .unwrap_or_else(|_| EnvFilter::new("info"));

        let fmt_layer = fmt::layer()
            .with_writer(non_blocking)
            .with_target(false)
            .with_level(true)
            .with_ansi(false);

        let stdout_layer = fmt::layer()
            .with_target(false)
            .with_level(true)
            .with_ansi(cfg!(debug_assertions));

        let subscriber = Registry::default()
            .with(env_filter)
            .with(fmt_layer)
            .with(stdout_layer);

        tracing::subscriber::set_global_default(subscriber)?;

        info!("logger initialized at {:?}", log_dir);
        Ok(())
    }
}

fn default_log_dir(app: &tauri::AppHandle) -> anyhow::Result<PathBuf> {
    #[allow(deprecated)]
    let base = app
        .path()
        .app_cache_dir()
        .unwrap_or_else(|_| app.path().app_data_dir().unwrap_or(std::env::temp_dir()));
    Ok(base.join("logs"))
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LogInfo {
    pub dir: String,
    pub current_file: String,
    pub size_bytes: u64,
    pub backups: Vec<String>,
}

pub fn get_log_info(app: &tauri::AppHandle) -> anyhow::Result<LogInfo> {
    let dir = default_log_dir(app)?;
    let current = dir.join("app.log");
    let pattern = dir.join("app.log*");
    let mut backups = vec![];
    for entry in glob::glob(pattern.to_string_lossy().as_ref())? {
        if let Ok(path) = entry { backups.push(path.to_string_lossy().to_string()); }
    }
    let size_bytes = std::fs::metadata(&current).map(|m| m.len()).unwrap_or(0);
    Ok(LogInfo {
        dir: dir.to_string_lossy().to_string(),
        current_file: current.to_string_lossy().to_string(),
        size_bytes,
        backups,
    })
}


