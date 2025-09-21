use std::fs;
use std::path::PathBuf;
use crate::config::Config;

/// 配置仓储接口
pub trait ConfigRepository: Send + Sync {
    fn load(&self) -> anyhow::Result<Config>;
    fn save(&self, config: &Config) -> anyhow::Result<()>;
}

/// 文件配置仓储实现
pub struct FileConfigRepository {
    config_path: PathBuf,
}

impl FileConfigRepository {
    pub fn new(config_path: PathBuf) -> Self {
        Self { config_path }
    }
}

impl ConfigRepository for FileConfigRepository {
    fn load(&self) -> anyhow::Result<Config> {
        if self.config_path.exists() {
            let data = fs::read_to_string(&self.config_path)?;
            let config = serde_json::from_str(&data).unwrap_or_default();
            Ok(config)
        } else {
            let default_config = Config::default();
            self.save(&default_config)?;
            Ok(default_config)
        }
    }

    fn save(&self, config: &Config) -> anyhow::Result<()> {
        if let Some(parent) = self.config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let data = serde_json::to_string(config)?;
        fs::write(&self.config_path, data)?;
        Ok(())
    }
}
