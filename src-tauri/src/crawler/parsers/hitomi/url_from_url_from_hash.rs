use crate::crawler::parsers::hitomi::gg_parser::GGRust;
use regex::Regex;

/// 图片结构体，对应JavaScript中的image对象
#[derive(Debug, Clone)]
pub struct Image {
    pub hash: String,
    pub name: Option<String>,
}

impl Image {
    pub fn new(hash: String, name: Option<String>) -> Self {
        Self { hash, name }
    }
}

/// 将hash转换为真实路径，对应JavaScript: real_full_path_from_hash
pub fn real_full_path_from_hash(hash: &str) -> String {
    if hash.len() < 3 {
        return "".to_string(); // 长度不够，无法截取
    }
    let chars: Vec<char> = hash.chars().collect();
    let last = chars[chars.len() - 1]; // 最后一个字符
    let second_last = chars[chars.len() - 2]; // 倒数第二个
    let third_last = chars[chars.len() - 3]; // 倒数第三个

    // 拼接路径：最后一个 / 倒数第三+倒数第二 / 原始hash
    format!(
        "{}/{}/{}",
        last,
        format!("{}{}", third_last, second_last),
        hash
    )
}

/// 从hash生成完整路径，对应JavaScript: full_path_from_hash
pub fn full_path_from_hash(gg: &GGRust, hash: &str) -> String {
    format!("{}{}/{}", gg.b, gg.s(hash), hash)
}

/// 从URL计算子域名，对应JavaScript: subdomain_from_url
pub fn subdomain_from_url(gg: &GGRust, url: &str, base: Option<&str>, dir: Option<&str>) -> String {
    let mut retval = String::new();

    // 首先检查dir参数设置特殊值
    if base.is_none() {
        if let Some(dir_str) = dir {
            if dir_str == "webp" {
                retval = "w".to_string();
            } else if dir_str == "avif" {
                retval = "a".to_string();
            }
        }
    }

    // 正则表达式匹配: /\/[0-9a-f]{61}([0-9a-f]{2})([0-9a-f])/
    let re = Regex::new(r"/[0-9a-f]{61}([0-9a-f]{2})([0-9a-f])").unwrap();

    if let Some(captures) = re.captures(url) {
        if let (Some(m1), Some(m2)) = (captures.get(1), captures.get(2)) {
            let g_str = format!("{}{}", m2.as_str(), m1.as_str());
            if let Ok(g) = u32::from_str_radix(&g_str, 16) {
                if let Some(base_str) = base {
                    let char_code = char::from_u32(97 + gg.m(g)).unwrap_or('a');
                    retval = format!("{}{}", char_code, base_str);
                } else {
                    retval = format!("{}{}", retval, 1 + gg.m(g));
                }
            }
        }
    }

    retval
}

/// 从URL生成最终URL，对应JavaScript: url_from_url
pub fn url_from_url(gg: &GGRust, url: &str, base: Option<&str>, dir: Option<&str>) -> String {
    let subdomain = subdomain_from_url(gg, url, base, dir);
    let domain = "gold-usergeneratedcontent.net/";

    // 替换域名部分
    let re = Regex::new(r"//..?\.(?:gold-usergeneratedcontent\.net|hitomi\.la)/").unwrap();
    re.replace(url, &format!("//{}.{}", subdomain, domain))
        .to_string()
}

/// 从hash生成URL，对应JavaScript: url_from_hash
pub fn url_from_hash(
    gg: &GGRust,
    galleryid: &str,
    image: &Image,
    dir: Option<&str>,
    ext: Option<&str>,
) -> String {
    let ext = ext
        .or(dir)
        .or_else(|| image.name.as_ref().and_then(|name| name.split('.').last()))
        .unwrap_or("jpg");

    let mut dir_path = dir.unwrap_or("").to_string();
    if dir_path == "webp" || dir_path == "avif" {
        dir_path = String::new();
    } else if !dir_path.is_empty() {
        dir_path.push('/');
    }

    let full_path = full_path_from_hash(gg, &image.hash);
    format!(
        "https://a.gold-usergeneratedcontent.net/{}{}{}.{}",
        dir_path, full_path, "", ext
    )
}

/// 主函数：从hash生成最终URL，对应JavaScript: url_from_url_from_hash
pub fn url_from_url_from_hash(
    gg: &GGRust,
    galleryid: &str,
    image: &Image,
    dir: Option<&str>,
    ext: Option<&str>,
    base: Option<&str>,
) -> String {
    if base == Some("tn") {
        let real_path = real_full_path_from_hash(&image.hash);
        let url = format!(
            "https://a.gold-usergeneratedcontent.net/{}/{}.{}",
            dir.unwrap_or(""),
            real_path,
            ext.unwrap_or("webp")
        );
        return url_from_url(gg, &url, base, None);
    }

    let url = url_from_hash(gg, galleryid, image, dir, ext);
    url_from_url(gg, &url, base, dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::crawler::parsers::hitomi::gg_parser::parse_gg_constants_rust;

    // 使用实际的gg.js内容进行测试
    const GG_JS_CONTENT: &str = include_str!("./gg.js");

    fn create_test_gg() -> GGRust {
        parse_gg_constants_rust(GG_JS_CONTENT).unwrap()
    }

    fn create_test_image(hash: &str) -> Image {
        Image::new(hash.to_string(), Some(format!("{}.jpg", hash)))
    }

    #[test]
    fn test_real_full_path_from_hash() {
        // 测试hash: "abc123def" -> "f/ef/abc123def"
        // JavaScript: hash.replace(/^.*(..)(.)$/, '$2/$1/' + hash)
        // (..)匹配最后两个字符"ef", (.)匹配最后一个字符"f"
        let result = real_full_path_from_hash("abc123def");
        assert_eq!(result, "f/ef/abc123def");

        // 测试短hash
        let result = real_full_path_from_hash("ab");
        assert_eq!(result, "ab");
    }

    #[test]
    fn test_full_path_from_hash() {
        let gg = create_test_gg();
        let hash = "fe0f3bc1b159625b4fe43d58e4a56b6b28b5353092ed5560a205bb5f651d8b3a";
        let result = full_path_from_hash(&gg, hash);

        // 应该包含gg.b和gg.s(hash)的结果
        assert!(result.contains(&gg.b));
        assert!(result.ends_with(hash));
    }

    #[test]
    fn test_subdomain_from_url() {
        let gg = create_test_gg();

        // 测试正常URL
        let url = "https://a.gold-usergeneratedcontent.net/1756044001/939/fe0f3bc1b159625b4fe43d58e4a56b6b28b5353092ed5560a205bb5f651d8b3a.jpg";
        let subdomain = subdomain_from_url(&gg, url, None, None);
        assert!(!subdomain.is_empty());

        // 测试webp格式
        let subdomain_webp = subdomain_from_url(&gg, url, None, Some("webp"));
        assert_eq!(subdomain_webp, "w");

        // 测试avif格式
        let subdomain_avif = subdomain_from_url(&gg, url, None, Some("avif"));
        assert_eq!(subdomain_avif, "a");
    }

    #[test]
    fn test_url_from_url() {
        let gg = create_test_gg();
        let url = "https://a.gold-usergeneratedcontent.net/1756044001/939/fe0f3bc1b159625b4fe43d58e4a56b6b28b5353092ed5560a205bb5f651d8b3a.jpg";

        let result = url_from_url(&gg, url, None, None);
        assert!(result.starts_with("https://"));
        assert!(result.contains("gold-usergeneratedcontent.net"));
    }

    #[test]
    fn test_url_from_hash() {
        let gg = create_test_gg();
        let image =
            create_test_image("fe0f3bc1b159625b4fe43d58e4a56b6b28b5353092ed5560a205bb5f651d8b3a");

        let result = url_from_hash(&gg, "12345", &image, None, Some("png"));
        assert!(result.starts_with("https://a.gold-usergeneratedcontent.net/"));
        assert!(result.ends_with(".png"));
        assert!(result.contains(&gg.b));
    }

    #[test]
    fn test_url_from_url_from_hash_normal() {
        let gg = create_test_gg();
        let image =
            create_test_image("fe0f3bc1b159625b4fe43d58e4a56b6b28b5353092ed5560a205bb5f651d8b3a");

        let result = url_from_url_from_hash(&gg, "12345", &image, Some(""), Some("jpg"), None);
        assert!(result.starts_with("https://"));
        assert!(result.contains("gold-usergeneratedcontent.net"));
        assert!(result.ends_with(".jpg"));
    }

    #[test]
    fn test_url_from_url_from_hash_thumbnail() {
        let gg = create_test_gg();
        let image =
            create_test_image("fe0f3bc1b159625b4fe43d58e4a56b6b28b5353092ed5560a205bb5f651d8b3a");

        let result =
            url_from_url_from_hash(&gg, "12345", &image, Some(""), Some("jpg"), Some("tn"));
        assert!(result.starts_with("https://"));
        assert!(result.contains("gold-usergeneratedcontent.net"));
        assert!(result.ends_with(".jpg"));

        // 缩略图应该使用不同的路径格式
        assert!(result.contains("/"));
    }

    #[test]
    fn test_edge_cases() {
        let gg = create_test_gg();

        // 测试空hash
        let image_empty = create_test_image("");
        let result = url_from_url_from_hash(&gg, "12345", &image_empty, None, Some("png"), None);
        assert!(result.ends_with(".png"));

        // 测试无扩展名的情况
        let image_no_ext = Image::new("hash123".to_string(), None);
        let result = url_from_url_from_hash(&gg, "12345", &image_no_ext, None, None, None);
        assert!(result.ends_with(".jpg")); // 默认扩展名
    }
}
