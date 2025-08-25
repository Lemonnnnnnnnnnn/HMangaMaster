use serde::Serialize;
use url::Url;

use crate::progress::ProgressReporter;
use crate::request::Client;
use rr::HeaderMap;

pub mod factory;
pub mod parsers;

use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct ParsedGallery {
    pub title: Option<String>,
    pub image_urls: Vec<String>,
    // 站点可选提供下载时需要附带的默认请求头（例如 Referer）。
    // 仅用于后端下载，不需要序列化给前端。
    #[serde(skip)]
    pub download_headers: Option<HeaderMap>,
    // 推荐的下载并发数，不设置则使用默认值
    #[serde(skip)]
    pub recommended_concurrency: Option<usize>,
}

// 解析器接口（统一为带 reporter 的单一方法，解析器可自由忽略 reporter）
#[allow(dead_code)]
pub trait SiteParser: Send + Sync {
    fn name(&self) -> &'static str;
    fn domains(&self) -> &'static [&'static str] {
        &[]
    }
    fn can_handle(&self, host: &str) -> bool {
        self.domains().iter().any(|d| host.ends_with(d))
    }
    fn parse<'a>(
        &'a self,
        client: &'a Client,
        url: &'a str,
        reporter: Option<Arc<dyn ProgressReporter>>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>,
    >;
}

// 解析器选择（可扩展：按 host 返回特定站点解析器）
fn ensure_builtin_registered() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        //  parsers 子模块注册
        crate::crawler::parsers::register_all();
    });
}

// 自动选择解析器并解析
pub async fn parse_gallery_auto(
    client: &Client,
    url: &str,
    reporter: Option<Arc<dyn ProgressReporter>>,
) -> anyhow::Result<ParsedGallery> {
    ensure_builtin_registered();
    let parsed = url
        .parse::<Url>()
        .map_err(|e| anyhow::anyhow!("无效的 URL: {}", e))?;
    let host = parsed.host_str().unwrap_or("").to_string();
    if let Some(site) = factory::detect_site_type_by_host(&host) {
        if let Some(parser) = factory::create_for_site(site) {
            return parser.parse(client, url, reporter).await;
        }
    }
    anyhow::bail!("未匹配到任何站点解析器，请检查 URL 或稍后重试")
}
