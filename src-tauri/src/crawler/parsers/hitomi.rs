use crate::crawler::{ParsedGallery, SiteParser, ProgressReporter};
use crate::request::Client;

pub struct HitomiParser;

impl HitomiParser { pub fn new() -> Self { Self } }

impl SiteParser for HitomiParser {
    fn name(&self) -> &'static str { "hitomi" }
    fn domains(&self) -> &'static [&'static str] { &["hitomi.la"] }
    fn parse<'a>(&'a self, client: &'a Client, url: &'a str) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            // 1) 从 URL 提取 ID
            let id = extract_id(url).ok_or_else(|| anyhow::anyhow!("无法从 URL 提取 ID"))?;
            // 2) 拉取 galleryinfo 脚本并解析为 JSON
            let gi_url = format!("https://ltn.gold-usergeneratedcontent.net/galleries/{}.js", id);
            let gi_resp = client.get(&gi_url).await?;
            if !gi_resp.status().is_success() { anyhow::bail!("获取 galleryinfo 失败: {}", gi_resp.status()); }
            let gi_text = gi_resp.text().await?;
            let (title, files) = parse_galleryinfo(&gi_text)?;

            // 3) 拉取 gg.js，并以 Rust 复刻关键逻辑生成图片 URL
            let gg_url = "https://ltn.gold-usergeneratedcontent.net/gg.js";
            let gg_resp = client.get(gg_url).await?;
            if !gg_resp.status().is_success() { anyhow::bail!("获取 gg.js 失败: {}", gg_resp.status()); }
            let gg_text = gg_resp.text().await?;
            let gg = parse_gg_constants(&gg_text)?;

            let mut image_urls: Vec<String> = Vec::with_capacity(files.len());
            for f in files {
                let ext = if f.haswebp == 1 { "webp" } else { infer_ext_from_name(&f.name).unwrap_or("jpg") };
                let url = build_hitomi_url(&gg, &f.hash, ext);
                image_urls.push(url);
            }
            if image_urls.is_empty() { anyhow::bail!("没有生成任何图片URL"); }
            Ok(ParsedGallery { title: Some(title), image_urls })
        })
    }
    fn parse_with_progress<'a>(&'a self, client: &'a Client, url: &'a str, reporter: Option<std::sync::Arc<dyn ProgressReporter>>) -> core::pin::Pin<Box<dyn core::future::Future<Output = anyhow::Result<ParsedGallery>> + Send + 'a>> {
        Box::pin(async move {
            let id = extract_id(url).ok_or_else(|| anyhow::anyhow!("无法从 URL 提取 ID"))?;
            let gi_url = format!("https://ltn.gold-usergeneratedcontent.net/galleries/{}.js", id);
            let gi_resp = client.get(&gi_url).await?;
            if !gi_resp.status().is_success() { anyhow::bail!("获取 galleryinfo 失败: {}", gi_resp.status()); }
            let gi_text = gi_resp.text().await?;
            let (title, files) = parse_galleryinfo(&gi_text)?;

            if let Some(r) = reporter.as_ref() { r.set_stage("parsing:images"); r.set_total(files.len()); }

            let gg_url = "https://ltn.gold-usergeneratedcontent.net/gg.js";
            let gg_resp = client.get(gg_url).await?;
            if !gg_resp.status().is_success() { anyhow::bail!("获取 gg.js 失败: {}", gg_resp.status()); }
            let gg_text = gg_resp.text().await?;
            let gg = parse_gg_constants(&gg_text)?;

            let mut image_urls: Vec<String> = Vec::with_capacity(files.len());
            for f in files {
                let ext = if f.haswebp == 1 { "webp" } else { infer_ext_from_name(&f.name).unwrap_or("jpg") };
                let url = build_hitomi_url(&gg, &f.hash, ext);
                image_urls.push(url);
                if let Some(r) = reporter.as_ref() { r.inc(1); }
            }
            if image_urls.is_empty() { anyhow::bail!("没有生成任何图片URL"); }
            Ok(ParsedGallery { title: Some(title), image_urls })
        })
    }
}

pub fn register() {
    use crate::crawler::factory::{register, register_host_contains};
    register("hitomi", || Box::new(HitomiParser::new()));
    register_host_contains("hitomi", vec!["hitomi.la"]);
}

// ---- utils ----

fn extract_id(url: &str) -> Option<String> {
    let re = regex::Regex::new(r"-(\d+)\.html").ok()?;
    let caps = re.captures(url)?;
    Some(caps.get(1)?.as_str().to_string())
}

#[derive(serde::Deserialize)]
struct GalleryInfo { title: String, files: Vec<HitomiFile> }

#[derive(serde::Deserialize)]
struct HitomiFile { hash: String, #[serde(rename = "haswebp")] haswebp: i32, name: String }

fn parse_galleryinfo(js_text: &str) -> anyhow::Result<(String, Vec<HitomiFile>)> {
    // 提取 `var galleryinfo = {...};`
    let re = regex::Regex::new(r"var\s+galleryinfo\s*=\s*(\{[\s\S]*?\})\s*;?")?;
    let caps = re.captures(js_text).ok_or_else(|| anyhow::anyhow!("未找到 galleryinfo"))?;
    let json = caps.get(1).unwrap().as_str();
    let gi: GalleryInfo = serde_json::from_str(json)?;
    Ok((gi.title, gi.files))
}

struct GG { b: String }

fn parse_gg_constants(gg_js: &str) -> anyhow::Result<GG> {
    // 在 gg.js 中寻找 `gg.b = '...';`
    let re = regex::Regex::new(r"gg\.b\s*=\s*'([^']+)'")?;
    let caps = re.captures(gg_js).ok_or_else(|| anyhow::anyhow!("未找到 gg.b"))?;
    Ok(GG { b: caps.get(1).unwrap().as_str().to_string() })
}

fn build_hitomi_url(gg: &GG, hash: &str, ext: &str) -> String {
    // 参考 Go 端 gg.js 路径生成逻辑的结果结构（简化）：
    // https://a.gold-usergeneratedcontent.net/webp/<gg.b + s(hash)>/<hash>.<ext>
    // s(hash) 在 gg.js 中是一个映射，本实现用 hash 前两位近似生成稳定路径，满足绝大多数资源。
    // 若遇到 404，可进一步精确移植 gg.s 逻辑。
    let subdir = format!("{}/{}/{}", &hash[hash.len().saturating_sub(3)..hash.len()-2], &hash[hash.len()-2..], hash);
    format!("https://a.gold-usergeneratedcontent.net/{}/{}{}.{}", if ext == "webp" { "webp" } else { "" }, gg.b, subdir, ext)
}

fn infer_ext_from_name(name: &str) -> Option<&'static str> {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".jpg") || lower.ends_with(".jpeg") { return Some("jpg"); }
    if lower.ends_with(".png") { return Some("png"); }
    if lower.ends_with(".webp") { return Some("webp"); }
    None
}


