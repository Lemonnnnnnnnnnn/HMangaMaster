use crate::batch_crawler::BatchCrawler;
use crate::request::Client;
use crate::batch_crawler::parsers::common::RequestContext;
use crate::config::service::ConfigService;
use reqwest::header::{HeaderMap, COOKIE};
use std::sync::Arc;

pub struct EhentaiBatchCrawler;

impl EhentaiBatchCrawler {
    pub fn new() -> Self {
        Self
    }

    /// 从单个页面提取漫画链接
    async fn extract_links_from_page(
        &self,
        request_ctx: &RequestContext,
        url: &str,
    ) -> anyhow::Result<(Vec<String>, Option<String>)> {
        let html = request_ctx.fetch_html(url).await?;
        let doc = scraper::Html::parse_document(&html);

        // 提取漫画链接: table.gltc tr td.gl3c.glname > a
        let mut manga_links = Vec::new();
        if let Ok(sel_table) = scraper::Selector::parse("table.gltc tr") {
            for tr in doc.select(&sel_table) {
                if let Ok(sel_gl3c) = scraper::Selector::parse("td.gl3c.glname a") {
                    for a in tr.select(&sel_gl3c) {
                        if let Some(href) = a.value().attr("href") {
                            if !href.is_empty() {
                                manga_links.push(href.to_string());
                            }
                        }
                    }
                }
            }
        }

        // 检查下一页链接: #dnext a
        let next_page = if let Ok(sel_dnext) = scraper::Selector::parse("#dnext a") {
            doc.select(&sel_dnext)
                .next()
                .and_then(|a| a.value().attr("href"))
                .map(|href| href.to_string())
        } else {
            None
        };

        Ok((manga_links, next_page))
    }

    /// 递归获取所有页面的漫画链接
    async fn extract_all_links(
        &self,
        request_ctx: &RequestContext,
        start_url: &str,
    ) -> anyhow::Result<Vec<String>> {
        let mut all_links = Vec::new();
        let mut current_url = start_url.to_string();

        loop {

            let (mut page_links, next_page) = self.extract_links_from_page(
                request_ctx,
                &current_url,
            ).await?;

            all_links.append(&mut page_links);

            match next_page {
                Some(next_url) if !next_url.is_empty() => {
                    current_url = next_url;
                    // 简单的延迟，避免请求过于频繁
                    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;
                }
                _ => break,
            }
        }

        // 去重，保持顺序
        {
            let mut seen = std::collections::HashSet::new();
            all_links.retain(|url| seen.insert(url.clone()));
        }

        Ok(all_links)
    }
}

impl BatchCrawler for EhentaiBatchCrawler {
    fn name(&self) -> &'static str {
        "ehentai_batch"
    }

    fn domains(&self) -> &'static [&'static str] {
        &["e-hentai.org", "exhentai.org"]
    }

    fn extract_manga_links<'a>(
        &'a self,
        client: &'a Client,
        url: &'a str,
        _reporter: Option<Arc<dyn crate::progress::ProgressReporter>>,
        app_state: Option<&'a crate::AppState>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<Vec<String>>> + Send + 'a>,
    > {
        Box::pin(async move {
            // 从配置中获取 parser 配置
            let parser_config = if let Some(state) = app_state {
                Some(state.config.read().get_parser_config("ehentai"))
            } else {
                None
            };

            // 使用配置中的并发数（对于批量解析，使用较低的并发数）
            let concurrency = parser_config
                .map(|config| config.base.concurrency)
                .flatten()
                .unwrap_or(3); // 批量解析默认使用较低的并发数

            let client_limited = client.with_limit(concurrency);
            let mut headers = HeaderMap::new();
            headers.insert(COOKIE, "nw=1".parse()?);

            let request_ctx = RequestContext::new(client_limited, headers, concurrency);

            // 提取所有页面的漫画链接
            let manga_links = self.extract_all_links(&request_ctx, url).await?;

            if manga_links.is_empty() {
                anyhow::bail!("未找到任何漫画链接");
            }

            Ok(manga_links)
        })
    }
}

pub fn register() {
    use crate::batch_crawler::factory::{register, register_host_contains};
    register("ehentai_batch", || Box::new(EhentaiBatchCrawler::new()));
    register_host_contains("ehentai_batch", vec!["e-hentai.org", "exhentai.org"]);
}