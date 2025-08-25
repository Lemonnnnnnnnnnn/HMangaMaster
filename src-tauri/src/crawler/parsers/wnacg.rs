use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::progress::ProgressContext;
use crate::request::Client;
use rr::HeaderMap;

pub struct WnacgParser;

impl WnacgParser {
    pub fn new() -> Self {
        Self
    }
}

impl SiteParser for WnacgParser {
    fn name(&self) -> &'static str {
        "wnacg"
    }
    fn domains(&self) -> &'static [&'static str] {
        &["wnacg.com", "www.wnacg.com"]
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
            let progress = ProgressContext::new(reporter, "Wnacg".to_string());

            // 使用限流请求
            let client_limited = client.with_limit(8); // 设置并发限制
            let headers = HeaderMap::new();
            let first = client_limited
                .get_with_headers_rate_limited(url, &headers)
                .await?;
            if !first.status().is_success() {
                anyhow::bail!("状态码异常: {}", first.status());
            }
            let html = first.text().await?;
            let (title_opt, mut page_urls): (Option<String>, Vec<String>) = {
                let doc = scraper::Html::parse_document(&html);
                let title = {
                    let sel = scraper::Selector::parse("#bodywrap > h2").unwrap();
                    doc.select(&sel)
                        .next()
                        .map(|n| n.text().collect::<String>())
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                };
                let mut urls = vec![url.to_string()];
                if let Ok(sel_pager) = scraper::Selector::parse(".paginator a") {
                    for a in doc.select(&sel_pager) {
                        if let Some(href) = a.value().attr("href") {
                            urls.push(to_abs_wnacg(href));
                        }
                    }
                }
                (title, urls)
            };
            page_urls.sort();
            page_urls.dedup();

            // 使用ProgressContext
            progress.update(0, page_urls.len(), "正在获取专辑页面");

            // 并发收集漫画详情页
            let mut manga_pages: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let results: Vec<Vec<String>> = stream::iter(page_urls.clone())
                    .map(|p| {
                        let client_cloned = client_limited.clone();
                        let headers_cloned = headers.clone();
                        async move {
                            let mut local: Vec<String> = vec![];
                            if let Ok(resp) = client_cloned
                                .get_with_headers_rate_limited(&p, &headers_cloned)
                                .await
                            {
                                if resp.status().is_success() {
                                    if let Ok(h) = resp.text().await {
                                        let d = scraper::Html::parse_document(&h);
                                        if let Ok(sel_item) =
                                            scraper::Selector::parse("#bodywrap ul li a")
                                        {
                                            for a in d.select(&sel_item) {
                                                if let Some(href) = a.value().attr("href") {
                                                    local.push(to_abs_wnacg(href));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            local
                        }
                    })
                    .buffer_unordered(8)
                    .collect()
                    .await;
                results.into_iter().flatten().collect()
            };
            manga_pages.sort();
            manga_pages.dedup();
            if manga_pages.is_empty() {
                anyhow::bail!("未找到任何漫画页面");
            }

            // 使用ProgressContext
            progress.update(0, manga_pages.len(), "正在解析图片链接");

            // 并发解析最终图片
            let mut image_urls: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let results: Vec<Vec<String>> = stream::iter(manga_pages.clone())
                    .map(|m| {
                        let client_cloned = client_limited.clone();
                        let headers_cloned = headers.clone();
                        async move {
                            let mut local: Vec<String> = vec![];
                            if let Ok(resp) = client_cloned
                                .get_with_headers_rate_limited(&m, &headers_cloned)
                                .await
                            {
                                if resp.status().is_success() {
                                    if let Ok(h) = resp.text().await {
                                        let d = scraper::Html::parse_document(&h);
                                        if let Ok(sel_img) = scraper::Selector::parse("#picarea") {
                                            for img in d.select(&sel_img) {
                                                if let Some(src) = img.value().attr("src") {
                                                    local.push(to_abs_wnacg(src));
                                                }
                                            }
                                        }
                                    }
                                }
                            }

                            local
                        }
                    })
                    .buffer_unordered(8)
                    .collect()
                    .await;
                results.into_iter().flatten().collect()
            };
            image_urls.sort();
            image_urls.dedup();
            if image_urls.is_empty() {
                anyhow::bail!("未解析到任何图片");
            }

            progress.set_message("解析完成，准备下载");

            Ok(ParsedGallery {
                title: title_opt,
                image_urls,
                download_headers: None,
                recommended_concurrency: None,
            })
        })
    }
}

fn to_abs_wnacg(u: &str) -> String {
    if u.starts_with("http://") || u.starts_with("https://") {
        return u.to_string();
    }
    if u.starts_with("//") {
        return format!("https:{}", u);
    }
    if u.starts_with('/') {
        return format!("https://www.wnacg.com{}", u);
    }
    format!("https://www.wnacg.com/{}", u.trim_start_matches("./"))
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("wnacg", || Box::new(WnacgParser::new()));
    register_host_contains("wnacg", vec!["wnacg.com"]);
}
