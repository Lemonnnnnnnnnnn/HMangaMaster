use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;
use crate::progress::ProgressContext;
use crate::crawler::parsers::common::RequestContext;
use crate::config::service::ConfigService;

pub struct Comic18Parser;

impl Comic18Parser {
    pub fn new() -> Self {
        Self
    }

}

impl SiteParser for Comic18Parser {
    fn name(&self) -> &'static str { "18comic" }
    fn domains(&self) -> &'static [&'static str] { &["18comic.vip", "18comic.org"] }
    fn parse<'a>(&'a self, client: &'a Client, url: &'a str, reporter: Option<std::sync::Arc<dyn ProgressReporter>>, app_state: Option<&'a crate::AppState>) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            // 创建ProgressContext
            let progress = ProgressContext::new(reporter, "18Comic".to_string());

            // 从配置中获取 parser 配置
            let parser_config = app_state.map(|state| state.config.read().get_parser_config("18comic"));

            // 使用配置中的并发数
            let concurrency = parser_config
                .and_then(|config| config.base.concurrency)
                .unwrap_or(5);
            let request_ctx = RequestContext::with_concurrency(client.clone(), concurrency);

            // 获取HTML内容
            let html = request_ctx.fetch_html(url).await?;
            let doc = scraper::Html::parse_document(&html);

            let title = {
                let sel = scraper::Selector::parse("h1").unwrap();
                doc.select(&sel).next().map(|n| n.text().collect::<String>()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
            };

            let sel_img = scraper::Selector::parse(".scramble-page > img").unwrap();
            let images: Vec<scraper::element_ref::ElementRef> = doc.select(&sel_img).collect();
            if images.is_empty() {
                anyhow::bail!("未找到任何图片");
            }

            // 使用ProgressContext
            progress.update(0, images.len(), "正在解析图片链接");

            let total_images = images.len();
            let mut image_urls: Vec<String> = Vec::with_capacity(total_images);
            for (i, img) in images.into_iter().enumerate() {
                if let Some(src) = img.value().attr("data-original") {
                    image_urls.push(src.to_string());
                } else if let Some(src) = img.value().attr("src") {
                    image_urls.push(src.to_string());
                }
                progress.update(i + 1, total_images, "正在解析图片链接");
            }

            image_urls.sort();
            image_urls.dedup();
            if image_urls.is_empty() {
                anyhow::bail!("未找到任何图片");
            }

            progress.set_message("解析完成，准备下载");

            Ok(ParsedGallery { title, image_urls, download_headers: None, recommended_concurrency: None })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("18comic", || Box::new(Comic18Parser::new()));
    register_host_contains("18comic", vec!["18comic.vip", "18comic.org"]);
}


