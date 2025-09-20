use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;
use crate::progress::ProgressContext;
use crate::crawler::parsers::common::RequestContext;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use serde_json::Value;
use scraper::{Html, Selector};
use anyhow::Context;
use regex;

pub struct PixivParser;

impl PixivParser {
    pub fn new() -> Self {
        Self
    }


    // 设置 Pixiv 特定的请求头和 cookies
    fn create_pixiv_headers(&self) -> HeaderMap {
        let mut headers = HeaderMap::new();

        // 设置 Referer
        headers.insert("referer", "https://www.pixiv.net/".parse().unwrap());

        // 设置 User-Agent
        headers.insert("user-agent", "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36".parse().unwrap());

        // 设置 Accept-Language
        headers.insert("accept-language", "en,zh-CN;q=0.9,zh;q=0.8".parse().unwrap());

        headers
    }


    // 从 URL 中提取作品 ID
    fn extract_artwork_id(&self, url: &str) -> anyhow::Result<String> {
        // 匹配 https://www.pixiv.net/artworks/130418478 这样的 URL
        if let Some(captures) = regex::Regex::new(r"/artworks/(\d+)")
            .unwrap()
            .captures(url)
        {
            Ok(captures.get(1).unwrap().as_str().to_string())
        } else {
            anyhow::bail!("无法从 URL 中提取作品 ID: {}", url);
        }
    }
}

impl SiteParser for PixivParser {
    fn name(&self) -> &'static str {
        "pixiv"
    }

    fn domains(&self) -> &'static [&'static str] {
        &["pixiv.net"]
    }

    fn parse<'a>(
        &'a self,
        client: &'a Client,
        url: &'a str,
        reporter: Option<std::sync::Arc<dyn ProgressReporter>>,
        app_state: Option<&'a crate::app::AppState>,
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            // 创建进度上下文
            let progress = ProgressContext::new(reporter, "Pixiv".to_string());

            // 提取作品 ID
            let artwork_id = self.extract_artwork_id(url)?;

            // 创建请求上下文，设置并发数为 1（避免请求过于频繁）
            let mut headers = self.create_pixiv_headers();

            // 从配置中获取 parser 配置
            let parser_config = if let Some(state) = app_state {
                Some(state.config.read().parser_config.get_config("pixiv"))
            } else {
                None
            };

            let cookies = parser_config
                .map(|config| config.auth)
                .flatten()
                .and_then(|auth| auth.cookies)
                .unwrap_or_default();

            if !cookies.is_empty() {
                headers.insert(COOKIE, HeaderValue::from_str(&cookies)?);
            }

            let request_ctx = RequestContext::new(client.clone(), headers, 1);

            progress.update(0, 100, "正在获取作品信息");

            // 获取作品页面 HTML
            let html = request_ctx.fetch_html(url).await?;

            // 解析 HTML 获取标题
            let title = {
                let doc = Html::parse_document(&html);
                let title_selector = Selector::parse("title").unwrap();
                doc
                    .select(&title_selector)
                    .next()
                    .map(|element| element.text().collect::<String>())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
            };

            progress.update(25, 100, "正在获取图片列表");

            // 构造 AJAX API URL
            let ajax_url = format!("https://www.pixiv.net/ajax/illust/{}/pages?lang=zh", artwork_id);

            // 获取图片数据
            let resp = request_ctx.client
                .get_with_headers_rate_limited(&ajax_url, &request_ctx.headers)
                .await?;

            if !resp.status().is_success() {
                anyhow::bail!("获取图片数据失败，状态码: {}", resp.status());
            }

            let json_text = resp.text().await?;
            let json_value: Value = serde_json::from_str(&json_text)
                .context("解析 JSON 响应失败")?;

            // 解析图片 URL
            let mut image_urls = Vec::new();
            if let Some(body) = json_value.get("body").and_then(|b| b.as_array()) {
                for page in body {
                    if let Some(urls) = page.get("urls") {
                        if let Some(regular_url) = urls.get("regular").and_then(|u| u.as_str()) {
                            image_urls.push(regular_url.to_string());
                        }
                    }
                }
            }

            if image_urls.is_empty() {
                anyhow::bail!("未找到任何图片");
            }

            progress.update(75, 100, "正在准备下载信息");

            // 设置下载请求头
            let mut download_headers = HeaderMap::new();
            download_headers.insert("referer", "https://www.pixiv.net/".parse().unwrap());

            // 推荐并发数为 1（避免请求过于频繁）
            let recommended_concurrency = Some(1);

            progress.update(100, 100, "解析完成");

            Ok(ParsedGallery {
                title,
                image_urls,
                download_headers: Some(download_headers),
                recommended_concurrency,
            })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("pixiv", || Box::new(PixivParser::new()));
    register_host_contains("pixiv", vec!["pixiv.net"]);
}
