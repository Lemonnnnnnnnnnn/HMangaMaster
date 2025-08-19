use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;

pub struct TelegraphParser;

impl TelegraphParser { pub fn new() -> Self { Self } }

impl SiteParser for TelegraphParser {
    fn name(&self) -> &'static str { "telegraph" }
    fn domains(&self) -> &'static [&'static str] { &["telegra.ph"] }
    fn parse<'a>(&'a self, client: &'a Client, url: &'a str, reporter: Option<std::sync::Arc<dyn ProgressReporter>>) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            let _ = reporter;
            let resp = client.get(url).await?;
            let html = resp.text().await?;
            let doc = scraper::Html::parse_document(&html);
            let sel_h1 = scraper::Selector::parse("h1").unwrap();
            let sel_img = scraper::Selector::parse("img").unwrap();

            let title = doc
                .select(&sel_h1)
                .next()
                .map(|h| h.text().collect::<String>())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            let mut images: Vec<String> = vec![];
            for (idx, img) in doc.select(&sel_img).enumerate() {
                if let Some(src) = img.value().attr("src") {
                    let full = normalize_telegraph_url(src);
                    if !full.is_empty() { images.push(full); }
                }
                if let Some(data) = img.value().attr("data-src") {
                    let full = normalize_telegraph_url(data);
                    if !full.is_empty() { images.push(full); }
                }
                // 生成固定顺序，避免重复
                let _ = idx; // keep enumeration for parity with Go naming if needed by caller
            }
            images.sort();
            images.dedup();

            if images.is_empty() { return Err(anyhow::anyhow!("未找到任何图片")); }
            Ok(ParsedGallery { title, image_urls: images, download_headers: None })
        })
    }
}

fn normalize_telegraph_url(u: &str) -> String {
    if u.starts_with("http://") || u.starts_with("https://") { return u.to_string(); }
    if u.starts_with('/') { return format!("https://telegra.ph{}", u); }
    String::new()
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("telegraph", || Box::new(TelegraphParser::new()));
    register_host_contains("telegraph", vec!["telegra.ph"]);
}


