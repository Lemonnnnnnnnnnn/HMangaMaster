use crate::progress::ProgressContext;
use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::request::Client;
use crate::crawler::parsers::common::RequestContext;
use crate::config::service::ConfigService;
use reqwest::header::{HeaderMap, COOKIE};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;



pub struct EhentaiParser;

impl EhentaiParser {
    pub fn new() -> Self {
        Self
    }


    async fn discover_pages(
        &self,
        request_ctx: &RequestContext,
        url: &str,
        progress: &ProgressContext,
    ) -> anyhow::Result<(Option<String>, Vec<String>)> {
        progress.set_message("正在获取专辑信息");

        let html = request_ctx.fetch_html(url).await?;
        let doc = scraper::Html::parse_document(&html);

        // 提取标题
        let title = {
            let sel = scraper::Selector::parse("#gn").unwrap();
            doc.select(&sel)
                .next()
                .map(|n| n.text().collect::<String>())
                .filter(|s| !s.trim().is_empty())
        };

        // 提取页面URLs
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

        // 稳定去重，保留首次出现的顺序
        {
            let mut seen = std::collections::HashSet::new();
            page_urls.retain(|u| seen.insert(u.clone()));
        }

        Ok((title, page_urls))
    }

    async fn extract_thumbnail_urls(
        &self,
        request_ctx: RequestContext,
        page_urls: Vec<String>,
        progress: ProgressContext,
    ) -> anyhow::Result<Vec<String>> {
        progress.update(0, page_urls.len(), "正在获取专辑页面");

        use futures_util::stream::{self, StreamExt};
        let pages_done = Arc::new(AtomicUsize::new(0));
        let total_pages = page_urls.len();

        let results: Vec<(usize, Vec<String>)> = stream::iter(page_urls.into_iter().enumerate())
            .map(|(idx, p)| {
                let headers = request_ctx.headers.clone();
                let client = request_ctx.client.clone();
                let progress = progress.clone();
                let done = pages_done.clone();

                async move {
                    let mut local: Vec<String> = vec![];
                    if let Ok(resp) = client.get_with_headers_rate_limited(&p, &headers).await {
                        if resp.status().is_success() {
                            if let Ok(html) = resp.text().await {
                                let doc = scraper::Html::parse_document(&html);
                                if let Ok(sel_gdt) = scraper::Selector::parse("#gdt > a") {
                                    for a in doc.select(&sel_gdt) {
                                        if let Some(href) = a.value().attr("href") {
                                            local.push(href.to_string());
                                        }
                                    }
                                }
                            }
                        }
                    }

                    let current = done.fetch_add(1, Ordering::Relaxed) + 1;
                    progress.update(current, total_pages, "正在获取专辑页面");
                    (idx, local)
                }
            })
            .buffer_unordered(request_ctx.concurrency)
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

        if flattened.is_empty() {
            anyhow::bail!("未找到任何图片链接");
        }

        Ok(flattened)
    }

    async fn resolve_image_urls(
        &self,
        request_ctx: RequestContext,
        thumbnail_urls: Vec<String>,
        progress: ProgressContext,
    ) -> anyhow::Result<Vec<String>> {
        progress.update(0, thumbnail_urls.len(), "正在解析图片链接");

        use futures_util::stream::{self, StreamExt};
        let imgs_done = Arc::new(AtomicUsize::new(0));
        let total_imgs = thumbnail_urls.len();

        let results: Vec<(usize, Option<String>)> = stream::iter(thumbnail_urls.into_iter().enumerate())
            .map(|(idx, tp)| {
                let headers = request_ctx.headers.clone();
                let client = request_ctx.client.clone();
                let progress = progress.clone();
                let done = imgs_done.clone();

                async move {
                    let resp = match client.get_with_headers_rate_limited(&tp, &headers).await {
                        Ok(v) => v,
                        Err(_) => return (idx, None),
                    };

                    if !resp.status().is_success() {
                        return (idx, None);
                    }

                    let html = match resp.text().await {
                        Ok(v) => v,
                        Err(_) => return (idx, None),
                    };

                    // 解析nl参数
                    let nl_val: Option<String> = {
                        let doc = scraper::Html::parse_document(&html);
                        if let Ok(sel_img) = scraper::Selector::parse("#img") {
                            if let Ok(re_nl) = regex::Regex::new(r"nl\('(.+?)'\)") {
                                let mut found: Option<String> = None;
                                if let Some(img) = doc.select(&sel_img).next() {
                                    if let Some(onerror) = img.value().attr("onerror") {
                                        if let Some(caps) = re_nl.captures(onerror) {
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

                    // 获取最终图片URL
                    let real_url = format!("{}?nl={}", &tp, nl);
                    let resp2 = match client.get_with_headers_rate_limited(&real_url, &headers).await {
                        Ok(v) => v,
                        Err(_) => return (idx, None),
                    };

                    if !resp2.status().is_success() {
                        return (idx, None);
                    }

                    let html2 = match resp2.text().await {
                        Ok(v) => v,
                        Err(_) => return (idx, None),
                    };

                    let doc2 = scraper::Html::parse_document(&html2);
                    let final_src = if let Ok(sel_img2) = scraper::Selector::parse("#img") {
                        doc2
                            .select(&sel_img2)
                            .next()
                            .and_then(|img2| img2.value().attr("src"))
                            .map(|s| s.to_string())
                    } else {
                        None
                    };

                    let current = done.fetch_add(1, Ordering::Relaxed) + 1;
                    progress.update(current, total_imgs, "正在解析图片链接");
                    (idx, final_src)
                }
            })
            .buffer_unordered(request_ctx.concurrency)
            .collect()
            .await;

        let mut ordered = results;
        ordered.sort_by_key(|(idx, _)| *idx);
        let image_urls: Vec<String> = ordered.into_iter().filter_map(|(_, v)| v).collect();

        if image_urls.is_empty() {
            anyhow::bail!("未解析到任何大图链接");
        }

        Ok(image_urls)
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
        app_state: Option<&'a crate::AppState>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>,
    > {
        Box::pin(async move {
            // 从配置中获取 parser 配置
            let parser_config = app_state.map(|state| state.config.read().get_parser_config("ehentai"));

            // 使用配置中的并发数
            let concurrency = parser_config
                .and_then(|config| config.base.concurrency)
                .unwrap_or(10);
            let client_limited = client.with_limit(concurrency);
            let mut headers = HeaderMap::new();
            headers.insert(COOKIE, "nw=1".parse()?);

            let request_ctx = RequestContext::new(client_limited, headers, concurrency);
            let progress = ProgressContext::new(reporter, "EHentai".to_string());

            // 1. 发现所有页面
            let (title, page_urls) = self.discover_pages(&request_ctx, url, &progress).await?;

            // 2. 提取缩略图
            let thumbnail_urls = self.extract_thumbnail_urls(request_ctx.clone(), page_urls, progress.clone()).await?;

            // 3. 解析大图URL
            let image_urls = self.resolve_image_urls(request_ctx, thumbnail_urls, progress.clone()).await?;

            // 4. 设置完成状态
            let final_message = match title.as_ref() {
                Some(t) if !t.is_empty() => format!("解析完成，准备下载: {}", t),
                _ => "解析完成，准备下载".to_string(),
            };
            progress.set_message(&final_message);

            Ok(ParsedGallery {
                title,
                image_urls,
                download_headers: None,
                recommended_concurrency: None,
            })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("ehentai", || Box::new(EhentaiParser::new()));
    register_host_contains("ehentai", vec!["e-hentai.org", "exhentai.org"]);
}
