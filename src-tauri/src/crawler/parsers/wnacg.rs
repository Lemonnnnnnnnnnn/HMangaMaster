use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::progress::ProgressContext;
use crate::request::Client;
use crate::config::service::ConfigService;
use reqwest::header::HeaderMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

#[derive(Debug, Clone)]
struct MangaDetail {
    detail_url: String,
    name: String,
}

// 从 gallary_wrap ul li 元素中解析漫画详情
fn parse_manga_detail_from_li(li: &scraper::ElementRef) -> Option<MangaDetail> {
    // 查找 img 元素获取 detail_url
    let img_selector = scraper::Selector::parse("a").ok()?;
    let img = li.select(&img_selector).next()?;

    let detail_url = img.value().attr("href")?;

    // 查找 class="name" 元素获取名称
    let name_selector = scraper::Selector::parse(".name").ok()?;
    let name_element = li.select(&name_selector).next()?;
    let name = name_element.text().collect::<String>().trim().to_string();

    if detail_url.is_empty() || name.is_empty() {
        tracing::debug!("跳过无效的漫画详情: detail_url='{}', name='{}'", detail_url, name);
        return None;
    }

    let abs_detail_url = to_abs_wnacg(detail_url);
    tracing::debug!("解析到漫画详情: name='{}', detail_url='{}'", name, abs_detail_url);

    Some(MangaDetail {
        detail_url: abs_detail_url,
        name,
    })
}

// 从详情页文档中提取第一个图片的 src
fn extract_first_image_src(doc: &scraper::Html) -> Option<String> {
    let sel_img = scraper::Selector::parse("#picarea").ok()?;
    let img = doc.select(&sel_img).next()?;

    if let Some(src) = img.value().attr("src") {
        tracing::debug!("从详情页提取到第一个图片URL: '{}'", src);
        Some(src.to_string())
    } else {
        tracing::warn!("详情页中未找到图片src属性");
        None
    }
}

// 解析图片URL模式，返回 (prefix, extension)
fn parse_image_url_pattern(first_src: &str, first_name: &str) -> Option<(String, String)> {
    tracing::debug!("解析图片URL模式: first_src='{}', first_name='{}'", first_src, first_name);

    if first_name.is_empty() {
        tracing::warn!("漫画名称为空，无法解析URL模式");
        return None;
    }

    // 查找名称在URL中的位置
    if let Some(name_pos) = first_src.find(first_name) {
        let prefix = first_src[..name_pos].to_string();
        let remaining = &first_src[name_pos + first_name.len()..];

        tracing::debug!("找到名称位置: name_pos={}, prefix='{}', remaining='{}'", name_pos, prefix, remaining);
        Some((prefix, ".webp".to_string()))
        // 查找扩展名（通常是 .webp, .jpg, .png 等）
        // if let Some(ext_start) = remaining.find('.') {
        //     let ext = remaining[ext_start..].to_string();
        //     tracing::debug!("解析成功: prefix='{}', extension='{}'", prefix, ext);
        //     Some((prefix, ext))
        // } else {
        //     tracing::warn!("在剩余部分 '{}' 中未找到扩展名", remaining);
        //     None
        // }
    } else {
        tracing::warn!("在URL '{}' 中未找到名称 '{}'", first_src, first_name);
        None
    }
}

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
        app_state: Option<&'a crate::AppState>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>,
    > {
        Box::pin(async move {
            tracing::info!("开始解析 WNACG 漫画: {}", url);

            // 创建ProgressContext
            let progress = ProgressContext::new(reporter, "Wnacg".to_string());

            // 从配置中获取 parser 配置
            let parser_config = if let Some(state) = app_state {
                Some(state.config.read().get_parser_config("wnacg"))
            } else {
                None
            };

            // 使用配置中的并发数
            let concurrency = parser_config
                .map(|config| config.base.concurrency)
                .flatten()
                .unwrap_or(3);
            
            let client_limited = client.with_limit(concurrency);
            let mut headers = HeaderMap::new();
            let _ = headers.insert("Referer", "https://www.wnacg.com/".parse().unwrap());
            let first = client_limited
                .get_with_headers_rate_limited(url, &headers)
                .await?;
            if !first.status().is_success() {
                anyhow::bail!("状态码异常: {}", first.status());
            }
            let html: String = first.text().await?;
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
                let urls = parse_wnacg_pagination(&doc, url);
                (title, urls)
            };
            page_urls.sort();
            page_urls.dedup();

            // 存储长度以避免在闭包中移动变量
            let page_urls_len = page_urls.len();

            tracing::info!("解析到标题: '{}', 分页数量: {}", title_opt.as_deref().unwrap_or("未知"), page_urls_len);
            tracing::debug!("分页URLs: {:?}", page_urls);

            // 使用ProgressContext
            progress.update(0, page_urls_len, "正在获取专辑页面");

            // 创建共享的进度计数器
            let progress_counter = Arc::new(AtomicUsize::new(0));
            let progress_arc = Arc::new(parking_lot::Mutex::new(progress.clone()));
            // 解析所有漫画详情页信息
            let manga_details: Vec<MangaDetail> = {
                use futures_util::stream::{self, StreamExt};
                let results: Vec<Vec<MangaDetail>> = stream::iter(page_urls.clone())
                    .map(|p| {
                        let client_cloned = client_limited.clone();
                        let base_url = url.to_string();
                        let counter = Arc::clone(&progress_counter);
                        let progress_clone = Arc::clone(&progress_arc);
                        let total_len = page_urls_len;
                        async move {
                            // 添加随机延迟
                            random_delay().await;

                            let mut local: Vec<MangaDetail> = vec![];
                            let mut headers_cloned = HeaderMap::new();
                            let _ = headers_cloned.insert("Referer", base_url.as_str().parse().unwrap());

                            if let Ok(resp) = client_cloned
                                .get_with_headers_rate_limited(&p, &headers_cloned)
                                .await
                            {
                                if resp.status().is_success() {
                                    if let Ok(h) = resp.text().await {
                                        let d = scraper::Html::parse_document(&h);
                                        // 解析 gallary_wrap 中的 li 元素
                                        if let Ok(sel_gallery) = scraper::Selector::parse(".gallary_wrap ul li") {
                                            tracing::debug!("使用 gallary_wrap 解析方式处理页面: {}", p);
                                            for li in d.select(&sel_gallery) {
                                                if let Some(detail) = parse_manga_detail_from_li(&li) {
                                                    local.push(detail);
                                                }
                                            }
                                        } else {
                                            // 降级到原有逻辑
                                            tracing::debug!("gallary_wrap 解析失败，降级到原有逻辑处理页面: {}", p);
                                            if let Ok(sel_item) = scraper::Selector::parse("#bodywrap ul li a") {
                                                for a in d.select(&sel_item) {
                                                    if let Some(href) = a.value().attr("href") {
                                                        let abs_url = to_abs_wnacg(href);
                                                        tracing::debug!("降级模式解析到详情URL: {}", abs_url);
                                                        local.push(MangaDetail {
                                                            detail_url: abs_url,
                                                            name: String::new(), // 降级模式下没有名称
                                                        });
                                                    }
                                                }
                                            }
                                        }
                                    }
                                } else {
                                    tracing::warn!("{}", resp.status());
                                }
                            }

                            // 更新进度
                            let current = counter.fetch_add(1, Ordering::SeqCst) + 1;
                            let progress_guard = progress_clone.lock();
                            progress_guard.update(current, total_len, "正在解析漫画详情");

                            local
                        }
                    })
                    .buffer_unordered(1)
                    .collect()
                    .await;
                results.into_iter().flatten().collect()
            };

            if manga_details.is_empty() {
                anyhow::bail!("未找到任何漫画详情");
            }

            // 存储长度以避免在闭包中移动变量
            let manga_details_len = manga_details.len();

            tracing::info!("解析到漫画详情数量: {}", manga_details_len);
            tracing::debug!("漫画详情列表: {:?}", manga_details.iter().map(|d| format!("name='{}', url='{}'", d.name, d.detail_url)).collect::<Vec<_>>());

            // 使用ProgressContext
            progress.update(0, manga_details_len, "正在计算图片链接");

            // 只访问第一个详情页来获取图片URL模式
            let first_detail = &manga_details[0];
            tracing::info!("访问第一个漫画详情页: name='{}', url='{}'", first_detail.name, first_detail.detail_url);

            let mut headers_cloned = HeaderMap::new();
            let _ = headers_cloned.insert("Referer", first_detail.detail_url.as_str().parse().unwrap());

            let first_resp = client_limited
                .get_with_headers_rate_limited(&first_detail.detail_url, &headers_cloned)
                .await?;

            if !first_resp.status().is_success() {
                anyhow::bail!("访问详情页失败: {}", first_resp.status());
            }

            let first_html: String = first_resp.text().await?;
            let first_doc = scraper::Html::parse_document(&first_html);
            tracing::debug!("成功获取第一个详情页HTML内容，长度: {}", first_html.len());

            // 从第一个详情页获取图片URL模式
            let image_urls = if let Some(first_src) = extract_first_image_src(&first_doc) {
                let first_src_abs = to_abs_wnacg(&first_src);
                tracing::debug!("第一个图片绝对URL: {}", first_src_abs);

                // 解析URL模式
                if let Some((prefix, ext)) = parse_image_url_pattern(&first_src_abs, &first_detail.name) {
                    // 使用模式生成所有图片URL
                    let urls: Vec<String> = manga_details.iter().map(|detail| {
                        if detail.name.is_empty() {
                            // 如果没有名称，使用原有逻辑访问详情页
                            // 这里为了简化，我们假设大部分情况下都有名称
                            let unknown_url = to_abs_wnacg(&format!("{}{}{}", prefix, "unknown", ext));
                            tracing::debug!("生成未知名称图片URL: {}", unknown_url);
                            unknown_url
                        } else {
                            let generated_url = to_abs_wnacg(&format!("{}{}{}", prefix, detail.name, ext));
                            tracing::debug!("生成图片URL: name='{}' -> {}", detail.name, generated_url);
                            generated_url
                        }
                    }).collect();

                    tracing::info!("使用URL模式生成图片URLs: prefix='{}', ext='{}', 数量: {}", prefix, ext, urls.len());
                    urls
                } else {
                    // 如果无法解析模式，降级到原有逻辑
                    tracing::warn!("无法解析图片URL模式，使用原有逻辑");
                    vec![first_src_abs]
                }
            } else {
                anyhow::bail!("无法从详情页获取图片URL");
            };

            tracing::info!("最终生成图片URL数量: {}", image_urls.len());

            if image_urls.is_empty() {
                anyhow::bail!("未解析到任何图片");
            }

            progress.set_message("解析完成（限速保护已生效），准备下载");

            tracing::info!("WNACG 解析完成: 标题='{}', 图片数量={}, 解析模式=优化模式",
                          title_opt.as_deref().unwrap_or("未知"),
                          image_urls.len());

            Ok(ParsedGallery {
                title: title_opt,
                image_urls,
                download_headers: None,
                recommended_concurrency: None,
            })
        })
    }
}

fn parse_wnacg_pagination(doc: &scraper::Html, base_url: &str) -> Vec<String> {
    let mut urls = vec![];
    tracing::debug!("解析分页: base_url={}", base_url);

    // 尝试解析分页器，获取最后一页的数字
    if let Ok(sel_pager) = scraper::Selector::parse(".paginator a") {
        let mut last_page_num = 1; // 默认至少有1页
        let mut page_numbers = vec![];

        // 查找所有分页链接，获取最后一页的数字
        for a in doc.select(&sel_pager) {
            if let Some(text) = a.text().next() {
                if let Ok(num) = text.trim().parse::<i32>() {
                    page_numbers.push(num);
                    if num > last_page_num {
                        last_page_num = num;
                    }
                }
            }
        }

        tracing::debug!("找到的分页数字: {:?}, 最大页数: {}", page_numbers, last_page_num);

        // 如果有多页，生成所有分页链接
        if last_page_num > 1 {
            urls.clear(); // 清空默认的URL

            // 解析基础URL结构
            // 例如: https://www.wnacg.com/photos-index-aid-317370.html
            // 需要找到 "-aid-" 的位置，在前面插入 "page-X"
            if let Some(aid_pos) = base_url.find("-aid-") {
                let base_part = &base_url[..aid_pos]; // https://www.wnacg.com/photos-index
                let suffix_part = &base_url[aid_pos..]; // -aid-317370.html

                tracing::debug!("URL结构解析: base_part='{}', suffix_part='{}'", base_part, suffix_part);

                // 生成所有分页链接
                for page_num in 1..=last_page_num {
                    let page_url = format!("{}-page-{}{}", base_part, page_num, suffix_part);
                    urls.push(page_url);
                }
                tracing::debug!("生成分页URLs: {}", urls.len());
            } else {
                tracing::warn!("无法解析URL结构: 未找到 '-aid-'");
                urls.push(base_url.to_string());
            }
        } else {
            tracing::debug!("只有1页或无法解析分页，使用原始URL");
            urls.push(base_url.to_string());
        }
    } else {
        tracing::debug!("未找到分页器，使用原始URL");
        urls.push(base_url.to_string());
    }

    tracing::debug!("最终分页URLs: {:?}", urls);
    urls
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

// 随机延迟函数，延迟 1-3 秒
async fn random_delay() {
    use rand::Rng;
    let delay_ms = rand::thread_rng().gen_range(1000..=3000);
    tokio::time::sleep(Duration::from_millis(delay_ms)).await;
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("wnacg", || Box::new(WnacgParser::new()));
    register_host_contains("wnacg", vec!["wnacg.com"]);
}
