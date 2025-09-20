use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::progress::ProgressContext;
use crate::request::Client;
use reqwest::header::HeaderMap;
use regex::Regex;
use url::form_urlencoded;

/// 图片转换策略
#[derive(Debug, Clone, Copy)]
enum ImageConversionStrategy {
    /// 转换为 webp 格式（11t.jpg -> 11.webp）
    Webp,
    /// 保持 jpg 格式（11t.jpg -> 11.jpg）
    Jpg,
}

pub struct NhentaiParser;

impl NhentaiParser {
    pub fn new() -> Self {
        Self
    }

}

impl SiteParser for NhentaiParser {
    fn name(&self) -> &'static str {
        "nhentai"
    }
    fn domains(&self) -> &'static [&'static str] {
        &["nhentai.net", "nhentai.xxx", "nhentai.to"]
    }
    fn parse<'a>(
        &'a self,
        client: &'a Client,
        url: &'a str,
        reporter: Option<std::sync::Arc<dyn ProgressReporter>>,
        app_state: Option<&'a crate::app::AppState>,
    ) -> core::pin::Pin<
        Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>,
    > {
        Box::pin(async move {
            // 创建ProgressContext
            let progress = ProgressContext::new(reporter, "NHentai".to_string());

            // 从URL中提取画廊ID
            let gallery_id = extract_gallery_id(url)?;
            tracing::debug!("提取到画廊ID: {}", gallery_id);

            // 从配置中获取 parser 配置
            let parser_config = if let Some(state) = app_state {
                Some(state.config.read().parser_config.get_config("nhentai"))
            } else {
                None
            };

            // 使用配置中的并发数
            let concurrency = parser_config
                .map(|config| config.base.concurrency)
                .flatten()
                .unwrap_or(5);
            let headers = HeaderMap::new();
            let client_limited = client.with_limit(concurrency);

            let resp = client_limited
                .get_with_headers_rate_limited(url, &headers)
                .await?;
            let html: String = resp.text().await?;

            // 解析HTML并提取数据
            let (title, thumbs, api_params) = parse_html_content(&html);

            if thumbs.is_empty() {
                anyhow::bail!("未找到任何图片");
            }

            tracing::debug!("从主页面获取到 {} 张缩略图URL", thumbs.len());

            // 使用ProgressContext
            progress.update(0, thumbs.len(), "正在解析图片链接");

            // 使用第一张图片确定转换策略
            let strategy = determine_conversion_strategy(&client_limited, &thumbs[0]).await;
            tracing::debug!("使用转换策略: {:?}", strategy);

            // 根据确定的策略转换所有缩略图URL
            let total_count = thumbs.len();
            let mut image_urls: Vec<String> = vec![];
            for (i, t) in thumbs.into_iter().enumerate() {
                image_urls.push(convert_nhentai_thumb(&t, strategy));
                progress.update(i + 1, total_count, "正在解析图片链接");
            }

            tracing::debug!("使用策略转换后获得 {} 张完整图片URL", image_urls.len());

            // 获取更多图片（通过AJAX接口）
            let more_images = if let Some(params) = api_params {
                get_more_images_from_api_with_params(
                    &client_limited,
                    &params,
                    image_urls.len(),
                    strategy,
                )
                .await
            } else {
                Ok(vec![])
            };

            match more_images {
                Ok(additional_images) => {
                    image_urls.extend(additional_images);
                    tracing::debug!("通过API获取到额外 {} 张图片URL", image_urls.len() - total_count);
                }
                Err(e) => {
                    tracing::warn!("获取更多图片失败: {}", e);
                }
            }

            progress.set_message("解析完成，准备下载");

            Ok(ParsedGallery {
                title,
                image_urls,
                download_headers: None,
                recommended_concurrency: None,
            })
        })
    }
}

/// 从URL中提取画廊ID
fn extract_gallery_id(gallery_url: &str) -> anyhow::Result<String> {
    // 从类似 "https://nhentai.xxx/g/537651/" 的URL中提取 "537651"
    let re = Regex::new(r"/g/(\d+)/?")?;
    if let Some(captures) = re.captures(gallery_url) {
        if let Some(id) = captures.get(1) {
            return Ok(id.as_str().to_string());
        }
    }
    anyhow::bail!("无法从URL中提取画廊ID")
}

/// 将缩略图URL转换为完整图片URL
fn convert_nhentai_thumb(thumbnail_url: &str, strategy: ImageConversionStrategy) -> String {
    let re = Regex::new(r"(\d+)t\.jpg$").unwrap();
    match strategy {
        ImageConversionStrategy::Webp => {
            // 将结尾的【数字t.jpg】替换为【数字.webp】
            // 例如：http://i4.nhentaimg.com/016/9sazckpugf/11t.jpg -> http://i4.nhentaimg.com/016/9sazckpugf/11.webp
            re.replace(thumbnail_url, "$1.webp").to_string()
        }
        ImageConversionStrategy::Jpg => {
            // 将结尾的【数字t.jpg】替换为【数字.jpg】
            // 例如：http://i4.nhentaimg.com/016/9sazckpugf/11t.jpg -> http://i4.nhentaimg.com/016/9sazckpugf/11.jpg
            re.replace(thumbnail_url, "$1.jpg").to_string()
        }
    }
}

/// 测试图片URL的可访问性
async fn test_image_accessibility(client: &Client, image_url: &str) -> bool {
    // 发送HEAD请求测试图片是否可访问
    match client.head(image_url).await {
        Ok(resp) => resp.status().is_success(),
        Err(_) => false,
    }
}

/// 确定图片转换策略
async fn determine_conversion_strategy(
    client: &Client,
    first_thumbnail_url: &str,
) -> ImageConversionStrategy {
    // 尝试第一种策略：webp
    let webp_url = convert_nhentai_thumb(first_thumbnail_url, ImageConversionStrategy::Webp);
    tracing::debug!("测试WebP策略: {}", webp_url);
    if test_image_accessibility(client, &webp_url).await {
        tracing::debug!("WebP策略测试成功");
        return ImageConversionStrategy::Webp;
    }

    // 尝试第二种策略：jpg
    let jpg_url = convert_nhentai_thumb(first_thumbnail_url, ImageConversionStrategy::Jpg);
    tracing::debug!("测试JPG策略: {}", jpg_url);
    if test_image_accessibility(client, &jpg_url).await {
        tracing::debug!("JPG策略测试成功");
        return ImageConversionStrategy::Jpg;
    }

    // 如果都失败，默认使用webp策略
    tracing::debug!("所有策略测试失败，使用默认WebP策略");
    ImageConversionStrategy::Webp
}

/// API参数结构
#[derive(Debug)]
struct ApiParams {
    csrf_token: String,
    server: String,
    u_id: String,
    g_id: String,
    img_dir: String,
    total_pages: usize,
}

/// 解析HTML内容并提取所需数据
fn parse_html_content(html: &str) -> (Option<String>, Vec<String>, Option<ApiParams>) {
    let doc = scraper::Html::parse_document(html);
    
    // 提取标题
    let title = {
        let sel_name = scraper::Selector::parse(
            "body div.main_cnt div div.gallery_top div.info h1",
        )
        .unwrap();
        doc.select(&sel_name)
            .next()
            .map(|n| n.text().collect::<String>())
            .filter(|s| !s.trim().is_empty())
    };
    
    // 提取缩略图URLs
    let sel_img = scraper::Selector::parse("#thumbs_append > div > a > img").unwrap();
    let mut thumbs: Vec<String> = vec![];
    for img in doc.select(&sel_img) {
        if let Some(u) = img.value().attr("data-src") {
            thumbs.push(u.to_string());
        }
    }
    
    // 提取API参数
    let api_params = extract_api_params(&doc);
    
    (title, thumbs, api_params)
}

/// 从HTML中提取API参数
fn extract_api_params(doc: &scraper::Html) -> Option<ApiParams> {
    // 获取CSRF token
    let csrf_selector = scraper::Selector::parse(r#"meta[name="csrf-token"]"#).unwrap();
    let csrf_token = doc
        .select(&csrf_selector)
        .next()
        .and_then(|n| n.value().attr("content"))?
        .to_string();

    // 获取其他必需的参数
    let server_selector = scraper::Selector::parse("#load_server").unwrap();
    let server = doc
        .select(&server_selector)
        .next()
        .and_then(|n| n.value().attr("value"))?
        .to_string();

    let uid_selector = scraper::Selector::parse("#gallery_id").unwrap();
    let u_id = doc
        .select(&uid_selector)
        .next()
        .and_then(|n| n.value().attr("value"))?
        .to_string();

    let gid_selector = scraper::Selector::parse("#load_id").unwrap();
    let g_id = doc
        .select(&gid_selector)
        .next()
        .and_then(|n| n.value().attr("value"))?
        .to_string();

    let img_dir_selector = scraper::Selector::parse("#load_dir").unwrap();
    let img_dir = doc
        .select(&img_dir_selector)
        .next()
        .and_then(|n| n.value().attr("value"))?
        .to_string();

    let total_pages_selector = scraper::Selector::parse("#load_pages").unwrap();
    let total_pages_str = doc
        .select(&total_pages_selector)
        .next()
        .and_then(|n| n.value().attr("value"))?;

    let total_pages: usize = total_pages_str.parse().ok()?;

    Some(ApiParams {
        csrf_token,
        server,
        u_id,
        g_id,
        img_dir,
        total_pages,
    })
}

/// 通过AJAX API获取更多图片
async fn get_more_images_from_api_with_params(
    client: &Client,
    params: &ApiParams,
    visible_pages: usize,
    strategy: ImageConversionStrategy,
) -> anyhow::Result<Vec<String>> {
    // 如果可见页面数量已经等于总页数，不需要调用API
    if visible_pages >= params.total_pages {
        return Ok(vec![]);
    }

    // 准备POST数据
    let form_data = form_urlencoded::Serializer::new(String::new())
        .append_pair("_token", &params.csrf_token)
        .append_pair("server", &params.server)
        .append_pair("u_id", &params.u_id)
        .append_pair("g_id", &params.g_id)
        .append_pair("img_dir", &params.img_dir)
        .append_pair("visible_pages", &visible_pages.to_string())
        .append_pair("total_pages", &params.total_pages.to_string())
        .append_pair("type", "2")
        .finish();

    // 设置请求头
    let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "application/x-www-form-urlencoded".parse()?);
    headers.insert("X-Requested-With", "XMLHttpRequest".parse()?);

    // 发送POST请求
    let resp = client
        .post_with_headers_rate_limited(
            "https://nhentai.xxx/modules/thumbs_loader.php",
            &headers,
            form_data,
        )
        .await?;

    if !resp.status().is_success() {
        anyhow::bail!("API返回错误状态码: {}", resp.status().as_u16());
    }

    let api_html = resp.text().await?.to_string();

    // 解析返回的HTML
    let api_doc = scraper::Html::parse_document(&api_html);
    let img_selector = scraper::Selector::parse("img").unwrap();

    // 从API响应中提取图片URL
    let mut more_images = Vec::new();
    for img in api_doc.select(&img_selector) {
        if let Some(data_src) = img.value().attr("data-src") {
            if !data_src.is_empty() {
                // 转换缩略图URL为完整图片URL
                let full_image_url = convert_nhentai_thumb(data_src, strategy);
                more_images.push(full_image_url);
            }
        }
    }

    Ok(more_images)
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("nhentai", || Box::new(NhentaiParser::new()));
    register_host_contains("nhentai", vec!["nhentai.net", "nhentai.xxx", "nhentai.to"]);
}
