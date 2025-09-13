use rr::{ClientBuilder, HeaderMap, HttpClient, ProxyConfig, Response};
use std::sync::Arc;
use tokio::sync::Semaphore;

#[derive(Clone)]
pub struct Client {
    http: HttpClient,
    default_headers: HeaderMap,
    limiter: Arc<Semaphore>,
}

const DEFAULT_CONCURRENCY: usize = 10;

impl Client {
    pub fn new(proxy_url: Option<String>) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7")?;
        headers.insert("accept-language", "en,zh-CN;q=0.9,zh;q=0.8")?;
        headers.insert("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")?;

        let mut builder = ClientBuilder::new().default_headers(headers.clone());

        if let Some(p) = proxy_url.filter(|s| !s.is_empty()) {
            let proxy_config = ProxyConfig::from_url(&p)?;
            builder = builder.proxy(proxy_config);
        }

        let http = builder.build()?;
        // 默认请求并发上限：10（可在特定站点覆盖）
        let limiter = Arc::new(Semaphore::new(DEFAULT_CONCURRENCY));
        Ok(Self {
            http,
            default_headers: headers,
            limiter,
        })
    }

    pub async fn get(&self, url: &str) -> anyhow::Result<Response> {
        Ok(self.http.get(url).send().await?)
    }

    pub async fn head(&self, url: &str) -> anyhow::Result<Response> {
        Ok(self.http.head(url).send().await?)
    }

    // 带并发限制的 GET
    pub async fn get_rate_limited(&self, url: &str) -> anyhow::Result<Response> {
        let _permit = self
            .limiter
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| anyhow::anyhow!("semaphore closed"))?;
        self.get(url).await
    }

    // 带并发限制与额外请求头的 GET
    pub async fn get_with_headers_rate_limited(
        &self,
        url: &str,
        headers: &HeaderMap,
    ) -> anyhow::Result<Response> {
        let _permit = self
            .limiter
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| anyhow::anyhow!("semaphore closed"))?;

        // 合并默认请求头和额外请求头
        let mut merged_headers = self.default_headers.clone();
        merged_headers.merge(headers);

        Ok(self
            .http
            .get(url)
            .headers_map(&merged_headers)
            .send()
            .await?)
    }

    // 带并发限制的 POST 请求
    pub async fn post_with_headers_rate_limited(
        &self,
        url: &str,
        headers: &HeaderMap,
        body: String,
    ) -> anyhow::Result<Response> {
        let _permit = self
            .limiter
            .clone()
            .acquire_owned()
            .await
            .map_err(|_| anyhow::anyhow!("semaphore closed"))?;

        // 合并默认请求头和额外请求头
        let mut merged_headers = self.default_headers.clone();
        merged_headers.merge(headers);

        Ok(self
            .http
            .post(url)
            .headers_map(&merged_headers)
            .body(body)
            .send()
            .await?)
    }

    // 返回一个设置了新并发上限的克隆客户端（不影响原实例）
    pub fn with_limit(&self, permits: usize) -> Self {
        Self {
            http: self.http.clone(),
            default_headers: self.default_headers.clone(),
            limiter: Arc::new(Semaphore::new(permits)),
        }
    }
}
