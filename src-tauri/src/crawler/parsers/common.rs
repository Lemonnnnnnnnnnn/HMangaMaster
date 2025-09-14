use crate::request::{RequestClient, Client};
use reqwest::header::HeaderMap;

/// 通用的请求上下文
/// 封装了网络请求相关的配置，支持复用
#[derive(Clone)]
pub struct RequestContext {
    pub client: RequestClient,
    pub headers: HeaderMap,
    pub concurrency: usize,
}

impl RequestContext {
    /// 创建新的请求上下文
    pub fn new(client: Client, headers: HeaderMap, concurrency: usize) -> Self {
        Self {
            client,
            headers,
            concurrency,
        }
    }

    /// 使用自定义并发数创建请求上下文
    pub fn with_concurrency(client: Client, concurrency: usize) -> Self {
        let headers = HeaderMap::new();
        Self::new(client, headers, concurrency)
    }

    /// 获取HTML内容
    pub async fn fetch_html(&self, url: &str) -> anyhow::Result<String> {
        let resp = self.client.get_with_headers_rate_limited(url, &self.headers).await?;
        if !resp.status().is_success() {
            anyhow::bail!("状态码异常: {}", resp.status());
        }
        resp.text().await.map_err(Into::into)
    }

}

/// URL标准化工具
pub mod url_utils {

    /// 标准化单个URL
    pub fn normalize_single_url(base_domain: &str, url: &str) -> Option<String> {
        let url = url.trim();

        if url.is_empty() {
            return None;
        }

        // 已经是完整URL
        if url.starts_with("http://") || url.starts_with("https://") {
            return Some(url.to_string());
        }

        // 协议相对URL
        if url.starts_with("//") {
            return Some(format!("https:{}", url));
        }

        // 绝对路径
        if url.starts_with('/') {
            return Some(format!("https://{}{}", base_domain, url));
        }

        // 相对路径
        if !url.contains("://") {
            return Some(format!("https://{}/{}", base_domain, url.trim_start_matches("./")));
        }

        None
    }

    /// 去重URL列表，保持首次出现的顺序
    pub fn deduplicate_urls(urls: Vec<String>) -> Vec<String> {
        let mut seen = std::collections::HashSet::new();
        urls.into_iter()
            .filter(|url| seen.insert(url.clone()))
            .collect()
    }
}
