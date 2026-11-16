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
            port: 8765,
            static_dir: Some("./web/dist".into()),
            cors_allowed_origins: vec!["*".into()],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum StorageType {
    Maildir,
    Sqlite,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct StorageEntry {
    pub name: String,
    #[serde(rename = "type")]
    pub storage_type: StorageType,
    pub enabled: bool,
    pub path: PathBuf,
}

impl StorageEntry {
    pub fn new_maildir<P: Into<PathBuf>>(name: &str, path: P) -> Self {
        Self {
            name: name.into(),
            storage_type: StorageType::Maildir,
            enabled: true,
            path: path.into(),
        }
    }

    pub fn new_sqlite<P: Into<PathBuf>>(name: &str, path: P) -> Self {
        Self {
            name: name.into(),
            storage_type: StorageType::Sqlite,
            enabled: true,
            path: path.into(),
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
            folder_path: "./tests/test_messages".into(),
            file_pattern: "*.json".into(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WahaAdapterConfig {
    pub enabled: bool,
    pub webhook_path: String,
    pub api_url: String,
    pub api_key: Option<String>,
}

impl Default for WahaAdapterConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            webhook_path: "/api/v1/webhook/waha".into(),
            api_url: "http://127.0.0.1:8768".into(),
            api_key: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AdaptersConfig {
    pub evolution: EvolutionAdapterConfig,
    pub waha: WahaAdapterConfig,
    pub folder: Option<FolderAdapterConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
    pub server: ServerConfig,
    pub storages: Vec<StorageEntry>,
    pub adapters: AdaptersConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            server: ServerConfig::default(),
            storages: vec![
                StorageEntry::new_maildir("maildir", "./data/mail"),
                StorageEntry::new_sqlite("sqlite", "./data/messages.sqlite3"),
            ],
            adapters: AdaptersConfig::default(),
        }
    }
}

impl AppConfig {
    pub fn get_storage(&self, name: &str) -> Option<&StorageEntry> {
        self.storages.iter().find(|s| s.name == name)
    }

    pub fn is_storage_enabled(&self, name: &str) -> bool {
        self.get_storage(name).map(|s| s.enabled).unwrap_or(false)
    }

    pub fn get_storage_path(&self, name: &str) -> Option<PathBuf> {
        self.get_storage(name).map(|s| s.path.clone())
    }

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
    fn test_config_storage_list() {
        let cfg = AppConfig::default();
        assert_eq!(cfg.storages.len(), 2);
        assert!(cfg.is_storage_enabled("maildir"));
        assert!(cfg.is_storage_enabled("sqlite"));

        let maildir_path = cfg.get_storage_path("maildir");
        assert_eq!(maildir_path, Some(PathBuf::from("./data/mail")));

        let toml_str = toml::to_string_pretty(&cfg).unwrap();
        assert!(toml_str.contains("[[storages]]"));
        assert!(toml_str.contains("name = \"maildir\""));

        let parsed: AppConfig = toml::from_str(&toml_str).unwrap();
        assert_eq!(cfg, parsed);
    }

    #[test]
    fn test_config_save_and_load() {
        let temp_file = std::env::temp_dir().join(format!("test_cfg_list_{}.toml", std::process::id()));
        let mut cfg = AppConfig::default();
        cfg.storages.push(StorageEntry {
            name: "backup_maildir".into(),
            storage_type: StorageType::Maildir,
            enabled: false,
            path: "./backup/mail".into(),
        });
        cfg.save_to_file(&temp_file).unwrap();

        let loaded = AppConfig::load_from_file(&temp_file).unwrap();
        assert_eq!(loaded.storages.len(), 3);
        assert!(!loaded.is_storage_enabled("backup_maildir"));

        let _ = fs::remove_file(temp_file);
    }
}
