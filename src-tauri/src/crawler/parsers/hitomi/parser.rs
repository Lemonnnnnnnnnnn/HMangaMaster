use crate::crawler::parsers::common::RequestContext;
use crate::crawler::{ParsedGallery, ProgressReporter, SiteParser};
use crate::progress::ProgressContext;
use crate::request::Client;
use reqwest::header::{HeaderMap, REFERER};

use super::gg_parser::parse_gg_constants_rust;
use super::url_from_url_from_hash::{url_from_url_from_hash, Image};
use super::utils::{extract_id, parse_galleryinfo};

pub struct HitomiParser;

impl HitomiParser {
    pub fn new() -> Self {
        Self
    }

}

impl SiteParser for HitomiParser {
    fn name(&self) -> &'static str {
        "hitomi"
    }

    fn domains(&self) -> &'static [&'static str] {
        &["hitomi.la"]
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
            // 创建ProgressContext
            let progress = ProgressContext::new(reporter, "Hitomi".to_string());

            let id = extract_id(url).ok_or_else(|| anyhow::anyhow!("无法从 URL 提取 ID"))?;

            // 从配置中获取 parser 配置
            let parser_config = if let Some(state) = app_state {
                Some(state.config.read().get_parser_config("hitomi"))
            } else {
                None
            };

            // 使用配置中的并发数
            let concurrency = parser_config
                .map(|config| config.base.concurrency)
                .flatten()
                .unwrap_or(3);
            let request_ctx = RequestContext::with_concurrency(client.clone(), concurrency);

            // 获取galleryinfo
            let gi_url = format!(
                "https://ltn.gold-usergeneratedcontent.net/galleries/{}.js",
                id
            );
            let gi_text = request_ctx.fetch_html(&gi_url).await?;
            let (title, files) = parse_galleryinfo(&gi_text)?;

            if files.is_empty() {
                anyhow::bail!("未找到任何文件信息");
            }

            // 使用ProgressContext
            progress.update(0, files.len(), "正在解析图片链接");

            // 获取gg.js
            let gg_url = "https://ltn.gold-usergeneratedcontent.net/gg.js";
            let gg_text = request_ctx.fetch_html(gg_url).await?;



            let gg = parse_gg_constants_rust(&gg_text)?;

            let total_files = files.len();
            let mut image_urls: Vec<String> = Vec::with_capacity(total_files);

            for (i, f) in files.into_iter().enumerate() {
                // 创建Image结构体用于URL生成
                let image = Image::new(f.hash.clone(), Some(f.name.clone()));

                // 使用url_from_url_from_hash生成URL，类似于Go版本的实现
                // Go版本调用: url_from_url_from_hash(galleryid, file, 'webp')
                let url = url_from_url_from_hash(
                    &gg,
                    &id,
                    &image,
                    Some("webp"), // dir参数，与Go版本保持一致
                    None,         // ext参数，Go版本中由file对象决定
                    None,         // base参数
                );
                image_urls.push(url.clone());

                progress.update(i + 1, total_files, "正在解析图片链接");
            }

            if image_urls.is_empty() {
                anyhow::bail!("没有生成任何图片URL");
            }

            progress.set_message("解析完成，准备下载");

            Ok(ParsedGallery {
                title: Some(title),
                image_urls,
                download_headers: {
                    let mut h = HeaderMap::new();
                    h.insert(REFERER, "https://hitomi.la/".parse()?);
                    Some(h)
                },
                recommended_concurrency: Some(4),
            })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("hitomi", || Box::new(HitomiParser::new()));
    register_host_contains("hitomi", vec!["hitomi.la"]);
}
