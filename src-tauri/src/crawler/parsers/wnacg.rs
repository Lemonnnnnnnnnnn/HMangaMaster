use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;

pub struct WnacgParser;

impl WnacgParser { pub fn new() -> Self { Self } }

impl SiteParser for WnacgParser {
    fn name(&self) -> &'static str { "wnacg" }
    fn domains(&self) -> &'static [&'static str] { &["wnacg.com", "www.wnacg.com"] }
    fn parse<'a>(&'a self, client: &'a Client, url: &'a str) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            // 1) 读取第一页，获取标题和分页链接
            let first = client.get(url).await?;
            if !first.status().is_success() { anyhow::bail!("状态码异常: {}", first.status()); }
            let html = first.text().await?;
            let (title_opt, mut page_urls): (Option<String>, Vec<String>) = {
                let doc = scraper::Html::parse_document(&html);
                let title = {
                    let sel = scraper::Selector::parse("#bodywrap > h2").unwrap();
                    doc.select(&sel).next().map(|n| n.text().collect::<String>()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
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

            // 2) 收集所有漫画页面链接（每一页的列表） - 并发
            let mut manga_pages: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let results: Vec<Vec<String>> = stream::iter(page_urls.clone())
                    .map(|p| {
                        let client_cloned = client.clone();
                        async move {
                        let mut local: Vec<String> = vec![];
                        if let Ok(resp) = client_cloned.get(&p).await {
                            if resp.status().is_success() {
                                if let Ok(h) = resp.text().await {
                                    let d = scraper::Html::parse_document(&h);
                                    if let Ok(sel_item) = scraper::Selector::parse("#bodywrap ul li a") {
                                        for a in d.select(&sel_item) {
                                            if let Some(href) = a.value().attr("href") { local.push(to_abs_wnacg(href)); }
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
            if manga_pages.is_empty() { anyhow::bail!("未找到任何漫画页面"); }

            // 3) 逐个漫画页面解析真实图片 URL（#picarea 的 img src） - 并发
            let mut image_urls: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let results: Vec<Vec<String>> = stream::iter(manga_pages.clone())
                    .map(|m| {
                        let client_cloned = client.clone();
                        // 无 reporter 可用（该函数签名无 reporter），仅并发抓取
                        async move {
                        let mut local: Vec<String> = vec![];
                        if let Ok(resp) = client_cloned.get(&m).await {
                            if resp.status().is_success() {
                                if let Ok(h) = resp.text().await {
                                    let d = scraper::Html::parse_document(&h);
                                    if let Ok(sel_img) = scraper::Selector::parse("#picarea") {
                                        for img in d.select(&sel_img) {
                                            if let Some(src) = img.value().attr("src") { local.push(to_abs_wnacg(src)); }
                                        }
                                    }
                                }
                            }
                        }
                        // 无进度回调
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
            if image_urls.is_empty() { anyhow::bail!("未解析到任何图片"); }

            Ok(ParsedGallery { title: title_opt, image_urls, download_headers: None })
        })
    }
    fn parse_with_progress<'a>(&'a self, client: &'a Client, url: &'a str, reporter: Option<std::sync::Arc<dyn ProgressReporter>>) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            let first = client.get(url).await?;
            if !first.status().is_success() { anyhow::bail!("状态码异常: {}", first.status()); }
            let html = first.text().await?;
            let (title_opt, mut page_urls): (Option<String>, Vec<String>) = {
                let doc = scraper::Html::parse_document(&html);
                let title = {
                    let sel = scraper::Selector::parse("#bodywrap > h2").unwrap();
                    doc.select(&sel).next().map(|n| n.text().collect::<String>()).map(|s| s.trim().to_string()).filter(|s| !s.is_empty())
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
            if let Some(r) = reporter.as_ref() { r.set_task_name(&format!("Wnacg - 正在获取专辑页面 (0/{}页)", page_urls.len())); r.set_total(page_urls.len()); }

            // 并发收集漫画详情页
            let mut manga_pages: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let results: Vec<Vec<String>> = stream::iter(page_urls.clone())
                    .map(|p| {
                        let client_cloned = client.clone();
                        async move {
                        let mut local: Vec<String> = vec![];
                        if let Ok(resp) = client_cloned.get(&p).await {
                            if resp.status().is_success() {
                                if let Ok(h) = resp.text().await {
                                    let d = scraper::Html::parse_document(&h);
                                    if let Ok(sel_item) = scraper::Selector::parse("#bodywrap ul li a") {
                                        for a in d.select(&sel_item) {
                                            if let Some(href) = a.value().attr("href") { local.push(to_abs_wnacg(href)); }
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
            if manga_pages.is_empty() { anyhow::bail!("未找到任何漫画页面"); }
            if let Some(r) = reporter.as_ref() { r.set_task_name(&format!("Wnacg - 正在解析图片链接 (0/{}张)", manga_pages.len())); r.set_total(manga_pages.len()); }

            // 并发解析最终图片
            let mut image_urls: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let results: Vec<Vec<String>> = stream::iter(manga_pages.clone())
                    .map(|m| {
                        let client_cloned = client.clone();
                        let rep = reporter.clone();
                        async move {
                        let mut local: Vec<String> = vec![];
                        if let Ok(resp) = client_cloned.get(&m).await {
                            if resp.status().is_success() {
                                if let Ok(h) = resp.text().await {
                                    let d = scraper::Html::parse_document(&h);
                                    if let Ok(sel_img) = scraper::Selector::parse("#picarea") {
                                        for img in d.select(&sel_img) {
                                            if let Some(src) = img.value().attr("src") { local.push(to_abs_wnacg(src)); }
                                        }
                                    }
                                }
                            }
                        }
                        if let Some(r) = rep.as_ref() { r.inc(1); }
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
            if image_urls.is_empty() { anyhow::bail!("未解析到任何图片"); }

            Ok(ParsedGallery { title: title_opt, image_urls, download_headers: None })
        })
    }
}

fn to_abs_wnacg(u: &str) -> String {
    if u.starts_with("http://") || u.starts_with("https://") { return u.to_string(); }
    if u.starts_with("//") { return format!("https:{}", u); }
    if u.starts_with('/') { return format!("https://www.wnacg.com{}", u); }
    format!("https://www.wnacg.com/{}", u.trim_start_matches("./"))
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("wnacg", || Box::new(WnacgParser::new()));
    register_host_contains("wnacg", vec!["wnacg.com"]);
}


