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

    // 获取 Pixiv 特定的 cookies
    fn get_pixiv_cookies(&self) -> &'static str {
        "first_visit_datetime_pc=2025-08-01%2021%3A11%3A23; p_ab_id=0; p_ab_id_2=5; p_ab_d_id=1185013192; yuid_b=N5cWMpg; PHPSESSID=39872424_qlqlIzTqNRmt7pLdmC77DGS3lylcrEBt; device_token=69a0afe143710692e8d1b566a0845948; c_type=26; privacy_policy_notification=0; a_type=0; b_type=1; privacy_policy_agreement=0; __cf_bm=b3fZXYQGLWWr_djvedcFnx6g8mSpqI_7QGG5cDER2GM-1758333122-1.0.1.1-hnW8BlJ_DpJTEzWNj.xUR2FmdpZeFlJ7rMMn6FC.c8FZmnSF9SErhRU.6283OIYwML.KR3BwwK_NpJqtKKs0HeCDZ8DPDXtldX7Q92bWSEyhS81x_4WPonm_sin1wNjB; _cfuvid=N735SX8oxaD5kcjDMvF5By.jmtPrgDxOxh41KPEk0y4-1758333122059-0.0.1.1-604800000; privacy_policy_agreement=7; cf_clearance=zUY4WNFr9g7D.csdp.infd8OZSUgvtEU9bgyAPTDEYc-1758333123-1.2.1.1-UBXfbDbkjwGkx8u6S8wLk7nbZE7vFVxphtqLnKs9c664rU0Y9OUZohyoSka8M7lSIoLy5EmEcWNoMyCgLkZGhN8xDA8lxYzwPQNWBM4HlgX.YUYe.yVUqLoLxiG9eqt2XY6lmRCyOZDqs.eM75DTyY7NEeVywhvujT0xlz1GH54VOtTY0gUE6QCIxjfqiTT1Kzi71iKbBKgOCqQ7eAxuvYUHMRlWmTyEcDOFVC6cpzE"
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
    ) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            // 创建进度上下文
            let progress = ProgressContext::new(reporter, "Pixiv".to_string());

            // 提取作品 ID
            let artwork_id = self.extract_artwork_id(url)?;

            // 创建请求上下文，设置并发数为 1（避免请求过于频繁）
            let mut headers = self.create_pixiv_headers();
            headers.insert(
                COOKIE,
                HeaderValue::from_static(self.get_pixiv_cookies())
            );

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
