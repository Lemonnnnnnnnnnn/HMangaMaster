use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::request::Client;
use reqwest::header::{HeaderMap, HeaderValue, COOKIE};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

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
            if let Some(r) = reporter.as_ref() {
                r.set_task_name("EHentai - 正在获取专辑信息");
            }
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
            // 稳定去重，保留首次出现的顺序
            {
                let mut seen = std::collections::HashSet::new();
                page_urls.retain(|u| seen.insert(u.clone()));
            }
            if let Some(r) = reporter.as_ref() {
                r.set_task_name(&format!(
                    "EHentai - 正在获取专辑页面 (0/{}页)",
                    page_urls.len()
                ));
                r.set_total(page_urls.len());
            }

            // 并发收集小图页
            let thumb_pages: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let pages_done = Arc::new(AtomicUsize::new(0));
                let total_pages = page_urls.len();
                let results: Vec<(usize, Vec<String>)> = stream::iter(page_urls.clone().into_iter().enumerate())
                    .map(|(idx, p)| {
                        let headers_cloned = headers.clone();
                        let client_cloned = client.clone();
                        let rep = reporter.clone();
                        let done = pages_done.clone();
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
                                r.set_task_name(&format!(
                                    "EHentai - 正在获取专辑页面 ({} / {}页)",
                                    cur,
                                    total_pages
                                ));
                            }
                            (idx, local)
                        }
                    })
                    .buffer_unordered(self.concurrency)
                    .collect()
                    .await;
                let mut ordered = results;
                ordered.sort_by_key(|(idx, _)| *idx);
                let mut flattened: Vec<String> = ordered.into_iter().flat_map(|(_, v)| v).collect();
                // 稳定去重，保留首次顺序
                {
                    let mut seen = std::collections::HashSet::new();
                    flattened.retain(|u| seen.insert(u.clone()));
                }
                flattened
            };
            if thumb_pages.is_empty() {
                anyhow::bail!("未找到任何图片链接");
            }
            if let Some(r) = reporter.as_ref() {
                r.set_task_name(&format!(
                    "EHentai - 正在解析图片链接 (0/{}张)",
                    thumb_pages.len()
                ));
                r.set_total(thumb_pages.len());
            }

            // 并发解析最终大图
            let image_urls: Vec<String> = {
                use futures_util::stream::{self, StreamExt};
                let imgs_done = Arc::new(AtomicUsize::new(0));
                let total_imgs = thumb_pages.len();
                let results: Vec<(usize, Option<String>)> = stream::iter(thumb_pages.clone().into_iter().enumerate())
                    .map(|(idx, tp)| {
                        let headers_cloned = headers.clone();
                        let client_cloned = client.clone();
                        let rep = reporter.clone();
                        let done = imgs_done.clone();
                        async move {
                            let resp = match client_cloned
                                .get_with_headers_rate_limited(&tp, &headers_cloned)
                                .await
                            {
                                Ok(v) => v,
                                Err(_) => return (idx, None),
                            };
                            if !resp.status().is_success() {
                                return (idx, None);
                            }
                            let h = match resp.text().await {
                                Ok(v) => v,
                                Err(_) => return (idx, None),
                            };
                            let nl_val: Option<String> = {
                                let d = scraper::Html::parse_document(&h);
                                if let Ok(sel_img) = scraper::Selector::parse("#img") {
                                    if let Ok(re_nl) = regex::Regex::new(r"nl\('(.+?)'\)") {
                                        let mut found: Option<String> = None;
                                        if let Some(img) = d.select(&sel_img).next() {
                                            if let Some(onerr) = img.value().attr("onerror") {
                                                if let Some(caps) = re_nl.captures(onerr) {
                                                    found = Some(caps.get(1).unwrap().as_str().to_string());
                                                }
                                            }
                                        }
                                        found
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            };
                            let nl = match nl_val {
                                Some(v) => v,
                                None => return (idx, None),
                            };
                            let real_url = format!("{}?nl={}", tp, nl);
                            let resp2 = match client_cloned
                                .get_with_headers_rate_limited(&real_url, &headers_cloned)
                                .await
                            {
                                Ok(v) => v,
                                Err(_) => return (idx, None),
                            };
                            if !resp2.status().is_success() {
                                return (idx, None);
                            }
                            let h2 = match resp2.text().await {
                                Ok(v) => v,
                                Err(_) => return (idx, None),
                            };
                            let d2 = scraper::Html::parse_document(&h2);
                            let final_src = if let Ok(sel_img2) = scraper::Selector::parse("#img") {
                                d2
                                    .select(&sel_img2)
                                    .next()
                                    .and_then(|img2| img2.value().attr("src"))
                                    .map(|s| s.to_string())
                            } else {
                                None
                            };
                            if let Some(r) = rep.as_ref() {
                                let cur = done.fetch_add(1, Ordering::Relaxed) + 1;
                                r.inc(1);
                                r.set_task_name(&format!(
                                    "EHentai - 正在解析图片链接 ({} / {}张)",
                                    cur, total_imgs
                                ));
                            }
                            (idx, final_src)
                        }
                    })
                    .buffer_unordered(self.concurrency)
                    .collect()
                    .await;
                let mut ordered = results;
                ordered.sort_by_key(|(idx, _)| *idx);
                ordered.into_iter().filter_map(|(_, v)| v).collect()
            };
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

            Ok(ParsedGallery {
                title,
                image_urls,
                download_headers: None,
            })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("ehentai", || Box::new(EhentaiParser::new()));
    register_host_contains("ehentai", vec!["e-hentai.org", "exhentai.org"]);
}
