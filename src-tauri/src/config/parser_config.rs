use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// 通用 parser 配置基类
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BaseParserConfig {
    pub concurrency: Option<usize>,
    pub timeout: Option<u64>,           // 毫秒
    pub task_concurrency: Option<usize>,      // 任务级并发数
    pub retry_count: Option<usize>,
    pub user_agent: Option<String>,
    pub custom_headers: HashMap<String, String>,
    pub proxy_enabled: bool,
}

/// 认证相关配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AuthConfig {
    pub cookies: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>,
    pub api_key: Option<String>,
    pub token: Option<String>,
}

/// 站点特定配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SiteSpecificConfig {
    pub settings: HashMap<String, serde_json::Value>,
}

/// 完整的 parser 配置
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ParserConfig {
    pub base: BaseParserConfig,
    pub auth: Option<AuthConfig>,
    pub site_specific: Option<SiteSpecificConfig>,
}

/// Parser 配置管理器
pub struct ParserConfigManager {
    configs: HashMap<String, ParserConfig>,
}

impl ParserConfigManager {
    pub fn new() -> Self {
        Self {
            configs: HashMap::new(),
        }
    }

    /// 获取指定 parser 的配置
    pub fn get_config(&self, parser_name: &str) -> ParserConfig {
        self.configs.get(parser_name)
            .cloned()
            .unwrap_or_else(|| {
                // 使用通用的默认配置
                let mut base = BaseParserConfig::default();
                base.task_concurrency = Some(3);  // 默认任务并发数
                base.concurrency = Some(3);  // 默认并发数
                ParserConfig {
                    base,
                    auth: None,
                    site_specific: None,
                }
            })
    }
}

impl ParserConfigManager {
    /// 设置指定 parser 的配置（公共 API）
    pub fn set_config(&mut self, parser_name: &str, config: ParserConfig) {
        self.configs.insert(parser_name.to_string(), config);
    }

    /// 获取所有配置（公共 API）
    pub fn get_all_configs(&self) -> &HashMap<String, ParserConfig> {
        &self.configs
    }
}

impl Default for ParserConfigManager {
    fn default() -> Self {
        Self::new()
    }
}
