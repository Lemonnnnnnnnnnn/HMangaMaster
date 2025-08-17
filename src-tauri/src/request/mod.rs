use reqwest::{header::HeaderMap, Proxy};

#[derive(Clone)]
pub struct Client {
    http: reqwest::Client,
    default_headers: HeaderMap,
}

impl Client {
    pub fn new(proxy_url: Option<String>) -> anyhow::Result<Self> {
        let mut headers = HeaderMap::new();
        headers.insert("accept", "text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,image/apng,*/*;q=0.8,application/signed-exchange;v=b3;q=0.7".parse()?);
        headers.insert("accept-language", "en,zh-CN;q=0.9,zh;q=0.8".parse()?);
        headers.insert("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse()?);

        let mut builder = reqwest::Client::builder()
            .default_headers(headers.clone())
            .http2_adaptive_window(true)
            .use_rustls_tls()
            .cookie_store(true)
            .pool_max_idle_per_host(8)
            .pool_idle_timeout(std::time::Duration::from_secs(30));

        if let Some(p) = proxy_url.filter(|s| !s.is_empty()) { builder = builder.proxy(Proxy::all(p)?); }

        let http = builder.build()?;
        Ok(Self { http, default_headers: headers })
    }

    pub async fn get(&self, url: &str) -> anyhow::Result<reqwest::Response> { Ok(self.http.get(url).send().await?) }

    pub async fn head(&self, url: &str) -> anyhow::Result<reqwest::Response> { Ok(self.http.head(url).send().await?) }

    pub async fn post_form(&self, url: &str, form: &[(impl AsRef<str>, impl AsRef<str>)]) -> anyhow::Result<reqwest::Response> {
        let mut pairs: Vec<(String, String)> = Vec::with_capacity(form.len());
        for (k, v) in form.iter() { pairs.push((k.as_ref().to_string(), v.as_ref().to_string())); }
        Ok(self.http.post(url)
            .header("X-Requested-With", "XMLHttpRequest")
            .form(&pairs)
            .send().await?)
    }

    pub async fn get_with_headers(&self, url: &str, headers: &HeaderMap) -> anyhow::Result<reqwest::Response> {
        let mut req = self.http.get(url);
        for (k, v) in headers.iter() { req = req.header(k, v); }
        Ok(req.send().await?)
    }
}


