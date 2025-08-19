use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::request::Client;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use std::sync::{Arc};
use std::sync::atomic::{AtomicUsize, Ordering};

pub struct EhentaiParser {
    concurrency: usize,
}

impl EhentaiParser {
    pub fn new() -> Self {
        Self { concurrency: 5 }
    }
}

impl SiteParser for EhentaiParser {
    fn name(&self) -> &'static str {
        "ehentai"
    }
    fn domains(&self) -> &'static [&'static str] {
        &["e-hentai.org", "exhentai.org"]
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
            // 覆写请求并发限制
            let client = client.with_limit(self.concurrency);
            let mut headers = HeaderMap::new();
            headers.insert(COOKIE, HeaderValue::from_static("nw=1"));
            if let Some(r) = reporter.as_ref() { r.set_task_name("EHentai - 正在获取专辑信息"); }
            let first = client.get_with_headers_rate_limited(url, &headers).await?;
            if !first.status().is_success() {
                anyhow::bail!("状态码异常: {}", first.status());
            }
            let html = first.text().await?;
            let (title, mut page_urls): (Option<String>, Vec<String>) = {
                let doc = scraper::Html::parse_document(&html);
                let title = {
                    let sel = scraper::Selector::parse("#gn").unwrap();
                    doc.select(&sel)
                        .next()
                        .map(|n| n.text().collect::<String>())
                        .filter(|s| !s.trim().is_empty())
                };
                let mut page_urls: Vec<String> = vec![url.to_string()];
                if let Ok(sel_gtb) = scraper::Selector::parse("body > .gtb") {
                    if let Some(gtb) = doc.select(&sel_gtb).next() {
                        let sel_td = scraper::Selector::parse("td a").unwrap();
                        for a in gtb.select(&sel_td) {
                            if let Some(href) = a.value().attr("href") {
                                page_urls.push(href.to_string());
                            }
                        }
                    }
                }
                (title, page_urls)
            };
            page_urls.sort();
            page_urls.dedup();
            if let Some(r) = reporter.as_ref() { r.set_task_name(&format!("EHentai - 正在获取专辑页面 (0/{}页)", page_urls.len())); r.set_total(page_urls.len()); }

            // 并发收集小图页
            let mut thumb_pages: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let pages_done = Arc::new(AtomicUsize::new(0));
                let results: Vec<Vec<String>> = stream::iter(page_urls.clone())
                    .map(|p| {
                        let headers_cloned = headers.clone();
                        let client_cloned = client.clone();
                        let rep = reporter.clone();
                        let done = pages_done.clone();
                        {
                        let value = page_urls.clone();
                        async move {
                            let mut local: Vec<String> = vec![];
                            if let Ok(resp) = client_cloned
                                .get_with_headers_rate_limited(&p, &headers_cloned)
                                .await
                            {
                                if resp.status().is_success() {
                                    if let Ok(h) = resp.text().await {
                                        let d = scraper::Html::parse_document(&h);
                                        if let Ok(sel_gdt) = scraper::Selector::parse("#gdt > a") {
                                            for a in d.select(&sel_gdt) {
                                                if let Some(href) = a.value().attr("href") {
                                                    local.push(href.to_string());
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            if let Some(r) = rep.as_ref() {
                                let cur = done.fetch_add(1, Ordering::Relaxed) + 1;
                                r.inc(1);
                                r.set_task_name(&format!("EHentai - 正在获取专辑页面 ({} / {}页)", cur, value.len()));
                            }
                            local
                        }
                        }
                    })
                    .buffer_unordered(8)
                    .collect()
                    .await;
                results.into_iter().flatten().collect()
            };
            thumb_pages.sort();
            thumb_pages.dedup();
            if thumb_pages.is_empty() {
                anyhow::bail!("未找到任何图片链接");
            }
            if let Some(r) = reporter.as_ref() { r.set_task_name(&format!("EHentai - 正在解析图片链接 (0/{}张)", thumb_pages.len())); r.set_total(thumb_pages.len()); }

            // 并发解析最终大图
            let mut image_urls: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let imgs_done = Arc::new(AtomicUsize::new(0));
                let total_imgs = thumb_pages.len();
                let results: Vec<Option<String>> = stream::iter(thumb_pages.clone())
                    .map(|tp| {
                        let headers_cloned = headers.clone();
                        let client_cloned = client.clone();
                        let rep = reporter.clone();
                        let done = imgs_done.clone();
                        async move {
                            let resp = client_cloned
                                .get_with_headers_rate_limited(&tp, &headers_cloned)
                                .await
                                .ok()?;
                            if !resp.status().is_success() {
                                return None;
                            }
                            let h = resp.text().await.ok()?;
                            let nl_val: Option<String> = {
                                let d = scraper::Html::parse_document(&h);
                                let sel_img = scraper::Selector::parse("#img").ok()?;
                                let re_nl = regex::Regex::new(r"nl\('(.+?)'\)").ok()?;
                                let mut found: Option<String> = None;
                                if let Some(img) = d.select(&sel_img).next() {
                                    if let Some(onerr) = img.value().attr("onerror") {
                                        if let Some(caps) = re_nl.captures(onerr) {
                                            found = Some(caps.get(1).unwrap().as_str().to_string());
                                        }
                                    }
                                }
                                found
                            };
                            let nl = nl_val?;
                            let real_url = format!("{}?nl={}", tp, nl);
                            let resp2 = client_cloned
                                .get_with_headers_rate_limited(&real_url, &headers_cloned)
                                .await
                                .ok()?;
                            if !resp2.status().is_success() {
                                return None;
                            }
                            let h2 = resp2.text().await.ok()?;
                            let d2 = scraper::Html::parse_document(&h2);
                            let sel_img2 = scraper::Selector::parse("#img").ok()?;
                            let final_src = d2
                                .select(&sel_img2)
                                .next()
                                .and_then(|img2| img2.value().attr("src"))
                                .map(|s| s.to_string());
                            if let Some(r) = rep.as_ref() {
                                let cur = done.fetch_add(1, Ordering::Relaxed) + 1;
                                r.inc(1);
                                r.set_task_name(&format!("EHentai - 正在解析图片链接 ({} / {}张)", cur, total_imgs));
                            }
                            final_src
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
                anyhow::bail!("未解析到任何大图链接");
            }
            if let Some(r) = reporter.as_ref() {
                let name = match title.as_ref() {
                    Some(t) if !t.is_empty() => format!("EHentai - 解析完成，准备下载: {}", t),
                    _ => "EHentai - 解析完成，准备下载".to_string(),
                };
                r.set_task_name(&name);
            }

            Ok(ParsedGallery { title, image_urls, download_headers: None })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("ehentai", || Box::new(EhentaiParser::new()));
    register_host_contains("ehentai", vec!["e-hentai.org", "exhentai.org"]);
}
