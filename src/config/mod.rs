use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub static_dir: Option<PathBuf>,
    pub cors_allowed_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8090,
            static_dir: Some("./web/dist".into()),
            cors_allowed_origins: vec!["*".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageConfig {
    pub data_dir: PathBuf,
    pub enable_maildir: bool,
    pub enable_db: bool,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            data_dir: "./data".into(),
            enable_maildir: true,
            enable_db: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvolutionAdapterConfig {
    pub enabled: bool,
    pub webhook_path: String,
}

impl Default for EvolutionAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            webhook_path: "/api/v1/webhook/evolution".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FolderAdapterConfig {
    pub enabled: bool,
    pub folder_path: PathBuf,
    pub file_pattern: String,
}

impl Default for FolderAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            folder_path: "./test_messages".into(),
            file_pattern: "*.json".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AdaptersConfig {
    pub evolution: EvolutionAdapterConfig,
    pub folder: Option<FolderAdapterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub storage: StorageConfig,
    pub adapters: AdaptersConfig,
}

impl AppConfig {
    pub fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let content = fs::read_to_string(path.as_ref())
            .with_context(|| format!("Failed to read config file {:?}", path.as_ref()))?;
        let config: Self = toml::from_str(&content)
            .with_context(|| format!("Failed to parse TOML config from {:?}", path.as_ref()))?;
        Ok(config)
    }

    pub fn load_or_default(path_opt: Option<&Path>) -> Self {
        if let Some(path) = path_opt {
            if path.exists() {
                if let Ok(cfg) = Self::load_from_file(path) {
                    return cfg;
                }
            }
        }

        let default_path = Path::new("mymessages.toml");
        if default_path.exists() {
            if let Ok(cfg) = Self::load_from_file(default_path) {
                return cfg;
            }
        }

        Self::default()
    }

    pub fn save_to_file<P: AsRef<Path>>(&self, path: P) -> Result<()> {
        let content = toml::to_string_pretty(self)?;
        fs::write(path.as_ref(), content)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_defaults_and_serialization() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.server.port, 8090);
        assert!(cfg.storage.enable_maildir);
        assert!(cfg.storage.enable_db);

        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        assert!(toml_str.contains("[server]"));
        assert!(toml_str.contains("cors_allowed_origins"));

        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_file = std::env::temp_dir().join(format!("test_cfg_{}.toml", std::process::id()));
        let mut cfg = AppConfig::default();
        cfg.server.port = 9999;
        cfg.save_to_file(&temp_file).unwrap();

        let loaded = AppConfig::load_from_file(&temp_file).unwrap();
        assert_eq!(loaded.server.port, 9999);

        let _ = fs::remove_file(temp_file);
    }
}
