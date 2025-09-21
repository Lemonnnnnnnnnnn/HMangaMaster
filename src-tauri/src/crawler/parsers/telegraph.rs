use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;
use crate::progress::ProgressContext;
use crate::crawler::parsers::common::{RequestContext, url_utils};
use crate::config::service::ConfigService;

pub struct TelegraphParser;

impl TelegraphParser {
    pub fn new() -> Self {
        Self
    }

}

impl SiteParser for TelegraphParser {
    fn name(&self) -> &'static str { "telegraph" }
    fn domains(&self) -> &'static [&'static str] { &["telegra.ph"] }
    fn parse<'a>(&'a self, client: &'a Client, url: &'a str, reporter: Option<std::sync::Arc<dyn ProgressReporter>>, app_state: Option<&'a crate::AppState>) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            // 创建ProgressContext
            let progress = ProgressContext::new(reporter, "Telegraph".to_string());

            // 从配置中获取 parser 配置
            let parser_config = if let Some(state) = app_state {
                Some(state.config.read().get_parser_config("telegraph"))
            } else {
                None
            };

            // 使用配置中的并发数
            let concurrency = parser_config
                .map(|config| config.base.concurrency)
                .flatten()
                .unwrap_or(3);
            let request_ctx = RequestContext::with_concurrency(client.clone(), concurrency);

            // 获取HTML内容
            let html = request_ctx.fetch_html(url).await?;
            let doc = scraper::Html::parse_document(&html);
            let sel_h1 = scraper::Selector::parse("h1").unwrap();
            let sel_img = scraper::Selector::parse("img").unwrap();

            let title = doc
                .select(&sel_h1)
                .next()
                .map(|h| h.text().collect::<String>())
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty());

            // 使用ProgressContext
            progress.set_message("正在解析图片链接");

            let mut raw_urls: Vec<String> = vec![];
            for (_, img) in doc.select(&sel_img).enumerate() {
                if let Some(src) = img.value().attr("src") {
                    if let Some(normalized) = url_utils::normalize_single_url("telegra.ph", src) {
                        raw_urls.push(normalized);
                    }
                }
                if let Some(data) = img.value().attr("data-src") {
                    if let Some(normalized) = url_utils::normalize_single_url("telegra.ph", data) {
                        raw_urls.push(normalized);
                    }
                }
            }

            // 去重和过滤
            let images = url_utils::deduplicate_urls(raw_urls);

            if images.is_empty() {
                anyhow::bail!("未找到任何图片");
            }

            progress.update(1, 1, "解析完成，准备下载");

            Ok(ParsedGallery { title, image_urls: images, download_headers: None, recommended_concurrency: None })
        })
    }
}



pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("telegraph", || Box::new(TelegraphParser::new()));
    register_host_contains("telegraph", vec!["telegra.ph"]);
}


