use std::path::{Path};
use tokio::io::AsyncWriteExt;

use crate::request::Client as RequestClient;
use rr::HeaderMap;
use tracing::{error, warn};

#[derive(Clone)]
pub struct Config { pub retry_count: usize, pub retry_delay_secs: u64 }
impl Default for Config { fn default() -> Self { Self { retry_count: 3, retry_delay_secs: 2 } } }

#[derive(Clone)]
pub struct Downloader { req: RequestClient, config: Config, default_headers: Option<HeaderMap> }

impl Downloader {
    // pub fn new(req: RequestClient, config: Config) -> Self { Self { req, config, default_headers: None } }
    pub fn new_with_headers(req: RequestClient, config: Config, headers: Option<HeaderMap>) -> Self { Self { req, config, default_headers: headers } }

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
                    let status = resp.status();
                    if !status.is_success() {
                        warn!(attempt = attempt + 1, status = %status, url = %url, "response is not successful");
                        last_err = Some(anyhow::anyhow!("bad status: {}", status));
                        continue;
                    }
                    // 将流写入文件的过程放入单独分支，错误不直接返回函数，而是记录并进入下一次重试
                    let write_res = async {
                        let mut file = tokio::fs::File::create(file_path).await?;
                        let mut stream = resp.bytes_stream();
                        use futures_util::StreamExt;
                        while let Some(chunk) = stream.next().await {
                            match chunk {
                                Ok(data) => {
                                    if let Err(e) = file.write_all(&data).await { return Err::<(), anyhow::Error>(e.into()); }
                                }
                                Err(e) => { return Err::<(), anyhow::Error>(e.into()); }
                            }
                        }
                        Ok::<(), anyhow::Error>(())
                    }.await;
                    match write_res {
                        Ok(()) => {
                            return Ok(());
                        }
                        Err(e) => {
                            warn!(attempt = attempt + 1, error = %e, "failed while writing response to file, will retry if attempts remain");
                            last_err = Some(e);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    warn!(attempt = attempt + 1, error = %e, "request failed, will retry if attempts remain");
                    last_err = Some(e.into());
                }
            }
        }
        error!(error = %last_err.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "unknown".to_string()), "all download attempts failed");
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
    // 简单的路径提取逻辑，避免依赖 url crate
    let path = if let Some(query_start) = url.find('?') {
        &url[..query_start]
    } else {
        url
    };

    let path_lower = if let Some(hash_start) = path.find('#') {
        path[..hash_start].to_ascii_lowercase()
    } else {
        path.to_ascii_lowercase()
    };
    if path_lower.ends_with(".webp") { return Some("webp"); }
    if path_lower.ends_with(".jpg") || path_lower.ends_with(".jpeg") { return Some("jpg"); }
    if path_lower.ends_with(".png") { return Some("png"); }
    if path_lower.ends_with(".gif") { return Some("gif"); }
    None
}


