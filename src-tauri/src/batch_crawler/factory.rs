use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;

use super::BatchCrawler;

type BatchCrawlerCtor = fn() -> Box<dyn BatchCrawler>;

pub type HostMatcher = Box<dyn Fn(&str) -> bool + Send + Sync + 'static>;

struct HostMatcherEntry {
    site_type: &'static str,
    matcher: HostMatcher,
}

static BATCH_CRAWLER_REGISTRY: Lazy<RwLock<HashMap<&'static str, BatchCrawlerCtor>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static HOST_MATCHERS: Lazy<RwLock<Vec<HostMatcherEntry>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

/// 注册批量解析器构造函数
pub fn register(site_type: &'static str, ctor: BatchCrawlerCtor) {
    BATCH_CRAWLER_REGISTRY.write().insert(site_type, ctor);
}

/// 注册主机匹配器
pub fn register_host_matcher(site_type: &'static str, matcher: HostMatcher) {
    HOST_MATCHERS
        .write()
        .push(HostMatcherEntry { site_type, matcher });
}

/// 注册基于域名包含的匹配器
pub fn register_host_contains(site_type: &'static str, substrings: Vec<&'static str>) {
    register_host_matcher(site_type, Box::new(move |host: &str| {
        substrings.iter().any(|s| {
            !s.is_empty() && host.to_ascii_lowercase().contains(&s.to_ascii_lowercase())
        })
    }));
}

/// 根据主机名检测站点类型
pub fn detect_site_type_by_host(host: &str) -> Option<&'static str> {
    for entry in HOST_MATCHERS.read().iter() {
        if (entry.matcher)(host) {
            return Some(entry.site_type);
        }
    }
    None
}

/// 为指定站点类型创建批量解析器
pub fn create_for_site(site_type: &str) -> Option<Box<dyn BatchCrawler>> {
    let reg = BATCH_CRAWLER_REGISTRY.read();
    let ctor = reg.get(site_type)?;
    Some(ctor())
}