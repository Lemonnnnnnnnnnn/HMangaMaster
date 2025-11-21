use url::Url;

use crate::progress::ProgressReporter;
use crate::request::Client;

pub mod factory;
pub mod parsers;

use std::sync::Arc;

/// 批量解析器接口
pub trait BatchCrawler: Send + Sync {

    /// 从列表页面提取所有漫画链接
    fn extract_manga_links<'a>(
        &'a self,
        client: &'a Client,
        url: &'a str,
        reporter: Option<Arc<dyn ProgressReporter>>,
        app_state: Option<&'a crate::AppState>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<Vec<String>>> + Send + 'a>,
    >;
}

/// 确保内置批量解析器已注册
fn ensure_builtin_registered() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        // 注册所有批量解析器
        crate::batch_crawler::parsers::register_all();
    });
}

/// 自动选择批量解析器并提取链接
pub async fn extract_manga_links_auto(
    client: &Client,
    url: &str,
    reporter: Option<Arc<dyn ProgressReporter>>,
    app_state: Option<&crate::AppState>,
) -> anyhow::Result<Vec<String>> {
    ensure_builtin_registered();
    let parsed = url
        .parse::<Url>()
        .map_err(|e| anyhow::anyhow!("无效的 URL: {}", e))?;
    let host = parsed.host_str().unwrap_or("").to_string();

    if let Some(site_type) = factory::detect_site_type_by_host(&host) {
        if let Some(crawler) = factory::create_for_site(site_type) {
            return crawler.extract_manga_links(client, url, reporter, app_state).await;
        }
    }
    anyhow::bail!("未匹配到任何批量解析器，请检查 URL 或稍后重试")
}