use std::path::{Path, PathBuf};
use tokio::io::AsyncWriteExt;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

use crate::request::Client as RequestClient;
use reqwest::header::HeaderMap;
use reqwest::Url;

#[derive(Clone)]
pub struct Config { pub retry_count: usize, pub retry_delay_secs: u64, pub concurrency: usize }
impl Default for Config { fn default() -> Self { Self { retry_count: 3, retry_delay_secs: 2, concurrency: 8 } } }

#[derive(Clone)]
pub struct Downloader { req: RequestClient, config: Config, default_headers: Option<HeaderMap> }

impl Downloader {
    pub fn new(req: RequestClient, config: Config) -> Self { Self { req, config, default_headers: None } }
    pub fn new_with_headers(req: RequestClient, config: Config, headers: Option<HeaderMap>) -> Self { Self { req, config, default_headers: headers } }
    // 允许外部覆写请求层的并发上限（例如站点特殊需求）
    pub fn set_request_limit(&mut self, permits: usize) { self.req.set_limit(permits); }

    pub async fn download_file(&self, url: &str, file_path: &Path) -> anyhow::Result<()> {
        if let Some(parent) = file_path.parent() { tokio::fs::create_dir_all(parent).await?; }

        let mut last_err: Option<anyhow::Error> = None;
        for attempt in 0..=self.config.retry_count {
            if attempt > 0 { tokio::time::sleep(std::time::Duration::from_secs(self.config.retry_delay_secs)).await; }
            let resp_result = match self.default_headers.as_ref() {
                Some(h) => self.req.get_with_headers_rate_limited(url, h).await,
                None => self.req.get_rate_limited(url).await,
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

}

// ---- helpers ----
pub fn build_download_plan(image_urls: &[String], base_path: &std::path::Path) -> (Vec<String>, Vec<std::path::PathBuf>) {
    let mut urls: Vec<String> = Vec::with_capacity(image_urls.len());
    let mut paths: Vec<std::path::PathBuf> = Vec::with_capacity(image_urls.len());
    for (idx, u) in image_urls.iter().enumerate() {
        urls.push(u.clone());
        let ext = infer_ext_from_url(u).unwrap_or("jpg");
        let filename = format!("{:04}.{}", idx + 1, ext);
        paths.push(base_path.join(&filename));
    }
    (urls, paths)
}

pub fn infer_ext_from_url(url: &str) -> Option<&'static str> {
    let path = Url::parse(url).ok()?.path().to_ascii_lowercase();
    if path.ends_with(".webp") { return Some("webp"); }
    if path.ends_with(".jpg") || path.ends_with(".jpeg") { return Some("jpg"); }
    if path.ends_with(".png") { return Some("png"); }
    if path.ends_with(".gif") { return Some("gif"); }
    None
}


