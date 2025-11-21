use crate::request::{RequestClient, Client};
use reqwest::header::HeaderMap;

/// 通用的请求上下文
/// 封装了网络请求相关的配置，支持复用
#[derive(Clone)]
pub struct RequestContext {
    pub client: RequestClient,
    pub headers: HeaderMap,
}

impl RequestContext {
    /// 创建新的请求上下文
    pub fn new(client: Client, headers: HeaderMap) -> Self {
        Self {
            client,
            headers,
        }
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
