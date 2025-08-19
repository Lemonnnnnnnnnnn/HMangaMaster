use serde::Serialize;

use crate::request::Client;
use reqwest::header::HeaderMap;

pub mod factory;
pub mod parsers;
pub mod reporter;

use std::sync::Arc;

#[derive(Debug, Clone, Serialize)]
pub struct ParsedGallery {
	pub title: Option<String>,
	pub image_urls: Vec<String>,
	// 站点可选提供下载时需要附带的默认请求头（例如 Referer）。
	// 仅用于后端下载，不需要序列化给前端。
	#[serde(skip)]
	pub download_headers: Option<HeaderMap>,
}

// 解析阶段进度上报（取消 stage 概念，允许解析器直接设置任务名）
pub trait ProgressReporter: Send + Sync {
    fn set_total(&self, _total: usize) {}
    fn inc(&self, _delta: usize) {}
    fn set_task_name(&self, _name: &str) {}
}

// 解析器接口
#[allow(dead_code)]
pub trait SiteParser: Send + Sync {
	fn name(&self) -> &'static str { "generic" }
	fn domains(&self) -> &'static [&'static str] { &[] }
	fn can_handle(&self, host: &str) -> bool {
		self.domains().iter().any(|d| host.ends_with(d))
	}
	fn parse<'a>(&'a self, client: &'a Client, url: &'a str) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>>;
	fn parse_with_progress<'a>(
		&'a self,
		client: &'a Client,
		url: &'a str,
		reporter: Option<Arc<dyn ProgressReporter>>,
	) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
		let _ = reporter; // 默认忽略进度
		self.parse(client, url)
	}
}

// 最小通用解析器
struct GenericParser;

impl GenericParser {
	fn new() -> Self { Self }
}

impl SiteParser for GenericParser {
	fn name(&self) -> &'static str { "generic" }
	fn parse<'a>(&'a self, client: &'a Client, url: &'a str) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
		Box::pin(async move {
			let resp = client.get(url).await?;
			let html = resp.text().await?;
			let (title, imgs) = {
				let doc = scraper::Html::parse_document(&html);
				let sel_img = scraper::Selector::parse("img").unwrap();
				let sel_title = scraper::Selector::parse("title").unwrap();
				let mut imgs: Vec<String> = vec![];
				for el in doc.select(&sel_img) {
					if let Some(src) = el.value().attr("src") {
						if is_http_url(src) { imgs.push(src.to_string()); }
					}
					if let Some(srcset) = el.value().attr("srcset") {
						if let Some(first) = srcset.split(',').next() {
							let url_part = first.trim().split_whitespace().next().unwrap_or("");
							if is_http_url(url_part) { imgs.push(url_part.to_string()); }
						}
					}
					if let Some(data) = el.value().attr("data-src") { if is_http_url(data) { imgs.push(data.to_string()); } }
					if let Some(data2) = el.value().attr("data-original") { if is_http_url(data2) { imgs.push(data2.to_string()); } }
				}
				imgs.sort();
				imgs.dedup();
				let title = doc
					.select(&sel_title)
					.next()
					.and_then(|t| Some(t.text().collect::<String>()))
					.map(|s| s.trim().to_string())
					.filter(|s| !s.is_empty());
				(title, imgs)
			};
			Ok(ParsedGallery { title, image_urls: imgs, download_headers: None })
		})
	}
}

// 解析器选择（可扩展：按 host 返回特定站点解析器）
fn ensure_builtin_registered() {
	use factory::register;
	static ONCE: std::sync::Once = std::sync::Once::new();
	ONCE.call_once(|| {
		// 注册 Generic
		register("generic", || Box::new(GenericParser::new()));
		// 其余站点交由 parsers 子模块注册
		crate::crawler::parsers::register_all();
	});
}

// 自动选择解析器并解析
pub async fn parse_gallery_auto(
    client: &Client,
    url: &str,
    reporter: Option<Arc<dyn ProgressReporter>>,
) -> anyhow::Result<ParsedGallery> {
	ensure_builtin_registered();
	let parsed = reqwest::Url::parse(url).map_err(|e| anyhow::anyhow!("无效的 URL: {}", e))?;
	let host = parsed.host_str().unwrap_or("").to_string();
	if let Some(site) = factory::detect_site_type_by_host(&host) {
		if let Some(parser) = factory::create_for_site(site) {
			return parser.parse_with_progress(client, url, reporter).await;
		}
	}
	anyhow::bail!("未匹配到任何站点解析器，请检查 URL 或稍后重试")
}


fn is_http_url(s: &str) -> bool { s.starts_with("http://") || s.starts_with("https://") }

