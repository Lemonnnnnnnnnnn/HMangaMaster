use crate::crawler::parsers::hitomi::types::{GalleryInfo, HitomiFile};
use regex::Regex;

/// 从URL中提取ID
pub fn extract_id(url: &str) -> Option<String> {
    let re = Regex::new(r"-(\d+)\.html").ok()?;
    let caps = re.captures(url)?;
    Some(caps.get(1)?.as_str().to_string())
}

/// 解析galleryinfo JavaScript
pub fn parse_galleryinfo(js_text: &str) -> anyhow::Result<(String, Vec<HitomiFile>)> {
    // 参考Go版本的正则表达式：var galleryinfo = (.+);?
    let re = Regex::new(r"var galleryinfo = (.+);?")?;
    let caps = re
        .captures(js_text)
        .ok_or_else(|| anyhow::anyhow!("未找到 galleryinfo"))?;
    let json = caps.get(1).unwrap().as_str();

    let gi: GalleryInfo = match serde_json::from_str(json) {
        Ok(gallery_info) => gallery_info,
        Err(e) => {
            return Err(anyhow::anyhow!("JSON 解析失败: {}", e));
        }
    };
    Ok((gi.title, gi.files))
}

