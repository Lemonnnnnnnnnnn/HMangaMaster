use once_cell::sync::Lazy;
use parking_lot::RwLock;
use std::collections::HashMap;

use super::SiteParser;

type ParserCtor = fn() -> Box<dyn SiteParser>;

pub type HostMatcher = Box<dyn Fn(&str) -> bool + Send + Sync + 'static>;

struct HostMatcherEntry {
    site_type: &'static str,
    matcher: HostMatcher,
}

static PARSER_REGISTRY: Lazy<RwLock<HashMap<&'static str, ParserCtor>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

static HOST_MATCHERS: Lazy<RwLock<Vec<HostMatcherEntry>>> =
    Lazy::new(|| RwLock::new(Vec::new()));

pub fn register(site_type: &'static str, ctor: ParserCtor) {
    PARSER_REGISTRY.write().insert(site_type, ctor);
}

pub fn register_host_matcher(site_type: &'static str, matcher: HostMatcher) {
    HOST_MATCHERS
        .write()
        .push(HostMatcherEntry { site_type, matcher });
}

pub fn register_host_contains(site_type: &'static str, substrings: Vec<&'static str>) {
    register_host_matcher(site_type, Box::new(move |host: &str| {
        substrings.iter().any(|s| {
            !s.is_empty() && host.to_ascii_lowercase().contains(&s.to_ascii_lowercase())
        })
    }));
}

pub fn detect_site_type_by_host(host: &str) -> Option<&'static str> {
    for entry in HOST_MATCHERS.read().iter() {
        if (entry.matcher)(host) {
            return Some(entry.site_type);
        }
    }
    None
}

pub fn create_for_site(site_type: &str) -> Option<Box<dyn SiteParser>> {
    let reg = PARSER_REGISTRY.read();
    let ctor = reg.get(site_type)?;
    Some(ctor())
}


