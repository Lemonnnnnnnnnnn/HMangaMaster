use std::sync::Arc;
use tokio_util::sync::CancellationToken;

use crate::crawler;
use crate::download;
use crate::progress;
use crate::request::Client;
use crate::task::manager::TaskManager;

/// 爬虫服务错误类型
#[derive(Debug)]
pub enum CrawlError {
    Cancelled,
    ParseFailed(String),
    ValidationFailed(String),
}

impl std::fmt::Display for CrawlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CrawlError::Cancelled => write!(f, "任务已取消"),
            CrawlError::ParseFailed(msg) => write!(f, "解析失败: {}", msg),
            CrawlError::ValidationFailed(msg) => write!(f, "验证失败: {}", msg),
        }
    }
}

impl std::error::Error for CrawlError {}

/// 爬虫服务
pub struct CrawlService;

impl CrawlService {
    /// 解析并验证URL
    pub async fn parse_and_validate(
        client: &Client,
        url: &str,
        task_id: &str,
        task_manager: &Arc<parking_lot::RwLock<TaskManager>>,
        cancel_token: &CancellationToken,
    ) -> Result<crawler::ParsedGallery, CrawlError> {
        // 创建进度报告器
        let reporter = Arc::new(progress::TaskReporter::new(
            task_id.to_string(),
            task_manager.clone(),
        ));

        // 解析阶段支持取消
        let parsed = tokio::select! {
            biased;
            _ = cancel_token.cancelled() => return Err(CrawlError::Cancelled),
            res = crawler::parse_gallery_auto(client, url, Some(reporter)) => {
                res.map_err(|e| CrawlError::ParseFailed(e.to_string()))?
            }
        };

        // 验证解析结果
        if parsed.image_urls.is_empty() {
            return Err(CrawlError::ValidationFailed("未解析到图片".to_string()));
        }

        Ok(parsed)
    }

    /// 构建下载计划
    pub fn build_download_plan(
        parsed: &crawler::ParsedGallery,
        output_dir: &str,
    ) -> (Vec<String>, Vec<std::path::PathBuf>) {
        let safe_name = sanitize_filename::sanitize(
            parsed
                .title
                .clone()
                .unwrap_or_else(|| "gallery".to_string()),
        );

        let base_path = std::path::PathBuf::from(output_dir).join(&safe_name);
        download::build_download_plan(&parsed.image_urls, &base_path)
    }

    /// 准备任务信息
    pub fn prepare_task_info(parsed: &crawler::ParsedGallery, output_dir: &str) -> (String, String) {
        let safe_name = sanitize_filename::sanitize(
            parsed
                .title
                .clone()
                .unwrap_or_else(|| "gallery".to_string()),
        );

        let base_path = std::path::PathBuf::from(output_dir).join(&safe_name);
        let save_path = base_path.to_string_lossy().to_string();

        (safe_name, save_path)
    }
}
