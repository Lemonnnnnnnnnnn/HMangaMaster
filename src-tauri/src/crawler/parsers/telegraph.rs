use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;
use crate::progress::ProgressContext;
use crate::crawler::parsers::common::{RequestContext, url_utils};
use crate::config::service::ConfigService;
use futures_util::stream::{self, StreamExt};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct TelegraphParser;

impl TelegraphParser {
    pub fn new() -> Self {
        Self
    }

    /// Extract sub-page links from a Telegraph main page.
    /// Parses all relative paths or telegraph.ph domain links in document order.
    fn extract_telegraph_subpage_links(doc: &scraper::Html, base_url: &str) -> Vec<String> {
        let base = url::Url::parse(base_url).unwrap_or_else(|_| url::Url::parse("https://telegra.ph").unwrap());
        let sel_a = scraper::Selector::parse("a").unwrap();

        let mut subpage_urls = Vec::new();

        for a in doc.select(&sel_a) {
            if let Some(href) = a.value().attr("href") {
                // Skip empty or anchor-only links
                if href.is_empty() || href.starts_with('#') {
                    continue;
                }

                // Use base.join() to handle both relative and absolute URLs
                if let Ok(joined) = base.join(href) {
                    // Only include telegraph.ph links (relative paths become telegraph.ph)
                    if joined.host_str().map_or(false, |host| host.ends_with("telegra.ph")) {
                        // Exclude the base URL itself
                        if joined.as_str() != base.as_str() {
                            subpage_urls.push(joined.to_string());
                        }
                    }
                }
            }
        }

        // Deduplicate while preserving order
        let mut seen = std::collections::HashSet::new();
        subpage_urls.retain(|url| seen.insert(url.clone()));

        tracing::debug!("Extracted {} sub-page links from main page", subpage_urls.len());
        subpage_urls
    }

    /// Parse images from a single Telegraph page.
    /// Extracts images from <img> elements with src and data-src attributes.
    async fn parse_telegraph_page_images(
        client: &Client,
        url: &str,
        base_domain: &str,
    ) -> anyhow::Result<Vec<String>> {
        // Fetch the page HTML
        let resp = client.get(url).await?;
        if !resp.status().is_success() {
            anyhow::bail!("Failed to fetch Telegraph page '{}': status {}", url, resp.status());
        }

        let html = resp.text().await?;
        let doc = scraper::Html::parse_document(&html);
        let sel_img = scraper::Selector::parse("img").unwrap();

        let mut raw_urls: Vec<String> = vec![];
        for img in doc.select(&sel_img) {
            if let Some(src) = img.value().attr("src") {
                if let Some(normalized) = url_utils::normalize_single_url(base_domain, src) {
                    raw_urls.push(normalized);
                }
            }
            if let Some(data) = img.value().attr("data-src") {
                if let Some(normalized) = url_utils::normalize_single_url(base_domain, data) {
                    raw_urls.push(normalized);
                }
            }
        }

        tracing::debug!("Parsed {} images from page '{}'", raw_urls.len(), url);
        Ok(raw_urls)
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
            let parser_config = app_state.map(|state| state.config.read().get_parser_config("telegraph"));

            // 使用配置中的并发数
            let concurrency = parser_config
                .and_then(|config| config.base.concurrency)
                .unwrap_or(3);
            let request_ctx = RequestContext::with_concurrency(client.clone(), concurrency);

            // 获取HTML内容
            let html = request_ctx.fetch_html(url).await?;

            // Extract title and sub-page links synchronously before any await
            // (scraper::Html is not Send, so we need to drop it before async operations)
            let (title, subpage_links, main_page_images): (Option<String>, Vec<String>, Vec<String>) = {
                let doc = scraper::Html::parse_document(&html);
                let sel_h1 = scraper::Selector::parse("h1").unwrap();
                let sel_img = scraper::Selector::parse("img").unwrap();

                let title = doc
                    .select(&sel_h1)
                    .next()
                    .map(|h| h.text().collect::<String>())
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty());

                let subpage_links = Self::extract_telegraph_subpage_links(&doc, url);

                // Extract images from the main page (for single-page case or as part of multi-page)
                let mut main_page_images: Vec<String> = Vec::new();
                for img in doc.select(&sel_img) {
                    if let Some(src) = img.value().attr("src") {
                        if let Some(normalized) = url_utils::normalize_single_url("telegra.ph", src) {
                            main_page_images.push(normalized);
                        }
                    }
                    if let Some(data) = img.value().attr("data-src") {
                        if let Some(normalized) = url_utils::normalize_single_url("telegra.ph", data) {
                            main_page_images.push(normalized);
                        }
                    }
                }

                // doc is dropped here
                (title, subpage_links, main_page_images)
            };

            let images = if !subpage_links.is_empty() {
                // Multi-page case: Parse all sub-pages concurrently
                tracing::info!("Found {} sub-pages, parsing concurrently", subpage_links.len());

                let progress_counter = Arc::new(AtomicUsize::new(0));
                let progress_arc = Arc::new(parking_lot::Mutex::new(progress.clone()));

                let total_pages = subpage_links.len() + 1; // +1 for main page
                tracing::debug!("Total pages to parse (including main page): {}", total_pages);

                progress.update(0, total_pages, "正在解析多页面内容");

                // Parse all sub-pages concurrently
                let results: Vec<anyhow::Result<Vec<String>>> = stream::iter(subpage_links)
                    .map(|page_url| {
                        let client_cloned = client.clone();
                        let counter = Arc::clone(&progress_counter);
                        let progress_clone = Arc::clone(&progress_arc);
                        let total = total_pages;
                        async move {
                            let page_images = Self::parse_telegraph_page_images(&client_cloned, &page_url, "telegra.ph").await?;

                            // Update progress
                            let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
                            progress_clone.lock().update(current, total, "正在解析子页面");

                            Ok::<Vec<String>, anyhow::Error>(page_images)
                        }
                    })
                    .buffered(concurrency)
                    .collect()
                    .await;

                // Combine main page images with all sub-page images
                let mut all_raw_urls: Vec<String> = main_page_images;
                for result in results {
                    match result {
                        Ok(page_images) => {
                            all_raw_urls.extend(page_images);
                        }
                        Err(e) => {
                            tracing::warn!("Failed to parse a Telegraph sub-page: {}", e);
                            // Continue with other pages even if one fails
                        }
                    }
                }

                tracing::info!("Collected {} total images from {} pages", all_raw_urls.len(), total_pages);
                url_utils::deduplicate_urls(all_raw_urls)
            } else {
                // Single-page case: Use images extracted from main page
                tracing::debug!("No sub-pages found, using single-page logic");
                progress.set_message("正在解析图片链接");

                url_utils::deduplicate_urls(main_page_images)
            };

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


