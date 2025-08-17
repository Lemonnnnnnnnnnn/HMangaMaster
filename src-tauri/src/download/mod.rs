use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::request::Client as RequestClient;
use reqwest::header::HeaderMap;

#[derive(Clone)]
pub struct Config { pub retry_count: usize, pub retry_delay_secs: u64, pub concurrency: usize }
impl Default for Config { fn default() -> Self { Self { retry_count: 3, retry_delay_secs: 2, concurrency: 8 } } }

#[derive(Clone)]
pub struct Downloader { req: RequestClient, config: Config, default_headers: Option<HeaderMap> }

impl Downloader {
    pub fn new(req: RequestClient, config: Config) -> Self { Self { req, config, default_headers: None } }
    pub fn new_with_headers(req: RequestClient, config: Config, headers: Option<HeaderMap>) -> Self { Self { req, config, default_headers: headers } }

    pub async fn download_file(&self, url: &str, file_path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = file_path.parent() { tokio::fs::create_dir_all(parent).await?; }

        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..=self.config.retry_count {
            if attempt > 0 { tokio::time::sleep(std::time::Duration::from_secs(self.config.retry_delay_secs)).await; }
            let resp_result = match self.default_headers.as_ref() {
                Some(h) => self.req.get_with_headers(url, h).await,
                None => self.req.get(url).await,
            };
            match resp_result {
                Ok(resp) => {
                    if !resp.status().is_success() { last_err = Some(anyhow::anyhow!("bad status: {}", resp.status())); continue; }
                    let mut file = tokio::fs::File::create(file_path).await?;
                    let mut stream = resp.bytes_stream();
                    use futures_util::StreamExt;
                    while let Some(chunk) = stream.next().await { let data = chunk?; file.write_all(&data).await?; }
                    return Ok(());
                }
                Err(e) => { last_err = Some(e.into()); }
            }
        }
        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("download failed")))
    }

    pub async fn download_many(&self, urls: &[String], paths: &[PathBuf], app: Option<tauri::AppHandle>, task_id: Option<String>) -> anyhow::Result<()> {
        self.download_many_with_token(urls, paths, app, task_id, None).await
    }

    pub async fn download_many_with_token(&self, urls: &[String], paths: &[PathBuf], app: Option<tauri::AppHandle>, task_id: Option<String>, cancel: Option<CancellationToken>) -> anyhow::Result<()> {
        if urls.len() != paths.len() { return Err(anyhow::anyhow!("urls 与 paths 数量不一致")); }
        let semaphore = std::sync::Arc::new(tokio::sync::Semaphore::new(self.config.concurrency));
        let mut tasks = vec![];
        for (idx, (u, p)) in urls.iter().zip(paths.iter()).enumerate() {
            let permit = semaphore.clone().acquire_owned().await?;
            let this = self.clone();
            let app_handle = app.clone();
            let u = u.clone();
            let p = p.clone();
            let tid = task_id.clone();
            let cancel_token = cancel.clone();
            let fut = tokio::spawn(async move {
                let _permit = permit;
                if let Some(ct) = cancel_token.as_ref() { if ct.is_cancelled() { return Err(anyhow::anyhow!("cancelled")); } }
                let res = this.download_file(&u, &p).await;
                if let Some(app) = app_handle {
                    let _ = app.emit("download:progress", serde_json::json!({"taskId": tid, "index": idx, "url": u, "ok": res.is_ok()}));
                }
                res
            });
            tasks.push(fut);
        }
        let results = futures_util::future::join_all(tasks).await;
        for r in results { r??; }
        if let Some(app) = app { let _ = app.emit("download:completed", serde_json::json!({"taskId": task_id})); }
        Ok(())
    }
}


