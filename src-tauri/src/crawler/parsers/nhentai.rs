use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;

pub struct NhentaiParser;

impl NhentaiParser { pub fn new() -> Self { Self } }

impl SiteParser for NhentaiParser {
    fn name(&self) -> &'static str { "nhentai" }
    fn domains(&self) -> &'static [&'static str] { &["nhentai.net", "nhentai.xxx", "nhentai.to"] }
    fn parse<'a>(&'a self, client: &'a Client, url: &'a str) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            let resp = client.get(url).await?;
            let html = resp.text().await?;
            let (title, thumbs) = {
                let doc = scraper::Html::parse_document(&html);
                let title = {
                    let sel_name = scraper::Selector::parse("body div.main_cnt div div.gallery_top div.info h1").unwrap();
                    doc.select(&sel_name).next().map(|n| n.text().collect::<String>()).filter(|s| !s.trim().is_empty())
                };
                let sel_img = scraper::Selector::parse("#thumbs_append > div > a > img").unwrap();
                let mut thumbs: Vec<String> = vec![];
                for img in doc.select(&sel_img) {
                    if let Some(u) = img.value().attr("data-src") { thumbs.push(u.to_string()); }
                }
                (title, thumbs)
            };
            if thumbs.is_empty() { anyhow::bail!("未找到任何图片"); }

            let first_webp = convert_nhentai_thumb(&thumbs[0], true);
            let webp_ok = client.head(&first_webp).await.map(|r| r.status().is_success()).unwrap_or(false);
            let use_webp = webp_ok;

            let mut image_urls: Vec<String> = vec![];
            for t in thumbs {
                image_urls.push(convert_nhentai_thumb(&t, use_webp));
            }

            Ok(ParsedGallery { title, image_urls, download_headers: None })
        })
    }
    fn parse_with_progress<'a>(&'a self, client: &'a Client, url: &'a str, reporter: Option<std::sync::Arc<dyn ProgressReporter>>) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            let resp = client.get(url).await?;
            let html = resp.text().await?;
            let (title, thumbs) = {
                let doc = scraper::Html::parse_document(&html);
                let title = {
                    let sel_name = scraper::Selector::parse("body div.main_cnt div div.gallery_top div.info h1").unwrap();
                    doc.select(&sel_name).next().map(|n| n.text().collect::<String>()).filter(|s| !s.trim().is_empty())
                };
                let sel_img = scraper::Selector::parse("#thumbs_append > div > a > img").unwrap();
                let mut thumbs: Vec<String> = vec![];
                for img in doc.select(&sel_img) {
                    if let Some(u) = img.value().attr("data-src") { thumbs.push(u.to_string()); }
                }
                (title, thumbs)
            };
            if thumbs.is_empty() { anyhow::bail!("未找到任何图片"); }
            if let Some(r) = reporter.as_ref() { r.set_task_name(&format!("NHentai - 正在解析图片链接 (0/{}张)", thumbs.len())); r.set_total(thumbs.len()); }

            let first_webp = convert_nhentai_thumb(&thumbs[0], true);
            let webp_ok = client.head(&first_webp).await.map(|r| r.status().is_success()).unwrap_or(false);
            let use_webp = webp_ok;

            let mut image_urls: Vec<String> = vec![];
            for t in thumbs {
                image_urls.push(convert_nhentai_thumb(&t, use_webp));
                if let Some(r) = reporter.as_ref() { r.inc(1); }
            }

            Ok(ParsedGallery { title, image_urls, download_headers: None })
        })
    }
}

fn convert_nhentai_thumb(thumbnail_url: &str, use_webp: bool) -> String {
    let re = regex::Regex::new(r"(\\d+)t\\.jpg$").unwrap();
    if use_webp { re.replace(thumbnail_url, "$1.webp").to_string() } else { re.replace(thumbnail_url, "$1.jpg").to_string() }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("nhentai", || Box::new(NhentaiParser::new()));
    register_host_contains("nhentai", vec!["nhentai.net", "nhentai.xxx", "nhentai.to"]);
}


