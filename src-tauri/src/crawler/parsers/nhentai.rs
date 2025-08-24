use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::progress::ProgressContext;
use crate::request::Client;
use rr::HeaderMap;

pub struct NhentaiParser;

impl NhentaiParser {
    pub fn new() -> Self {
        Self
    }
}

impl SiteParser for NhentaiParser {
    fn name(&self) -> &'static str {
        "nhentai"
    }
    fn domains(&self) -> &'static [&'static str] {
        &["nhentai.net", "nhentai.xxx", "nhentai.to"]
    }
    fn parse<'a>(
        &'a self,
        client: &'a Client,
        url: &'a str,
        reporter: Option<std::sync::Arc<dyn ProgressReporter>>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>,
    > {
        Box::pin(async move {
            // 创建ProgressContext
            let progress = ProgressContext::new(reporter, "NHentai".to_string());

            // 使用限流请求
            let headers = HeaderMap::new();
            let client_limited = client.with_limit(5); // 设置并发限制

            let resp = client_limited
                .get_with_headers_rate_limited(url, &headers)
                .await?;
            let html = resp.text().await?;

            let (title, thumbs) = {
                let doc = scraper::Html::parse_document(&html);
                let title = {
                    let sel_name = scraper::Selector::parse(
                        "body div.main_cnt div div.gallery_top div.info h1",
                    )
                    .unwrap();
                    doc.select(&sel_name)
                        .next()
                        .map(|n| n.text().collect::<String>())
                        .filter(|s| !s.trim().is_empty())
                };
                let sel_img = scraper::Selector::parse("#thumbs_append > div > a > img").unwrap();
                let mut thumbs: Vec<String> = vec![];
                for img in doc.select(&sel_img) {
                    if let Some(u) = img.value().attr("data-src") {
                        thumbs.push(u.to_string());
                    }
                }
                (title, thumbs)
            };

            if thumbs.is_empty() {
                anyhow::bail!("未找到任何图片");
            }

            // 使用ProgressContext
            progress.update(0, thumbs.len(), "正在解析图片链接");

            let first_webp = convert_nhentai_thumb(&thumbs[0], true);
            let webp_ok = client_limited
                .head(&first_webp)
                .await
                .map(|r| r.status().is_success())
                .unwrap_or(false);
            let use_webp = webp_ok;

            let total_count = thumbs.len();
            let mut image_urls: Vec<String> = vec![];
            for (i, t) in thumbs.into_iter().enumerate() {
                image_urls.push(convert_nhentai_thumb(&t, use_webp));
                progress.update(i + 1, total_count, "正在解析图片链接");
            }

            progress.set_message("解析完成，准备下载");

            Ok(ParsedGallery {
                title,
                image_urls,
                download_headers: None,
            })
        })
    }
}

fn convert_nhentai_thumb(thumbnail_url: &str, use_webp: bool) -> String {
    let re = regex::Regex::new(r"(\\d+)t\\.jpg$").unwrap();
    if use_webp {
        re.replace(thumbnail_url, "$1.webp").to_string()
    } else {
        re.replace(thumbnail_url, "$1.jpg").to_string()
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("nhentai", || Box::new(NhentaiParser::new()));
    register_host_contains("nhentai", vec!["nhentai.net", "nhentai.xxx", "nhentai.to"]);
}
