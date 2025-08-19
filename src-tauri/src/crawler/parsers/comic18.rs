use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;

pub struct Comic18Parser;

impl Comic18Parser { pub fn new() -> Self { Self } }

impl SiteParser for Comic18Parser {
    fn name(&self) -> &'static str { "18comic" }
    fn domains(&self) -> &'static [&'static str] { &["18comic.vip", "18comic.org"] }
    fn parse<'a>(&'a self, client: &'a Client, url: &'a str, reporter: Option<std::sync::Arc<dyn ProgressReporter>>) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            let resp = client.get(url).await?;
            if !resp.status().is_success() { anyhow::bail!("状态码异常: {}", resp.status()); }
            let html = resp.text().await?;
            let doc = scraper::Html::parse_document(&html);

            let title = {
                let sel = scraper::Selector::parse("h1").unwrap();
                doc.select(&sel).next().map(|n| n.text().collect::<String>()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
            };

            let sel_img = scraper::Selector::parse(".scramble-page > img").unwrap();
            let images: Vec<scraper::element_ref::ElementRef> = doc.select(&sel_img).collect();
            if images.is_empty() { anyhow::bail!("未找到任何图片"); }
            if let Some(r) = reporter.as_ref() { r.set_task_name(&format!("18Comic - 正在解析图片链接 (0/{}张)", images.len())); r.set_total(images.len()); }

            let mut image_urls: Vec<String> = Vec::with_capacity(images.len());
            for img in images {
                if let Some(src) = img.value().attr("data-original") { image_urls.push(src.to_string()); }
                else if let Some(src) = img.value().attr("src") { image_urls.push(src.to_string()); }
                if let Some(r) = reporter.as_ref() { r.inc(1); }
            }
            image_urls.sort();
            image_urls.dedup();
            if image_urls.is_empty() { anyhow::bail!("未找到任何图片"); }

            Ok(ParsedGallery { title, image_urls, download_headers: None })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("18comic", || Box::new(Comic18Parser::new()));
    register_host_contains("18comic", vec!["18comic.vip", "18comic.org"]);
}


