pub mod adapters;
pub mod credentials;

use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

pub use adapters::{
    AdaptersConfig, EvolutionAdapterConfig, FolderAdapterConfig, UbackAdapterConfig,
    WahaAdapterConfig,
};
pub use credentials::{expand_tilde, CredentialsStore};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub static_dir: Option<PathBuf>,
    #[serde(default)]
    pub tmp_dir: Option<PathBuf>,
    pub cors_allowed_origins: Vec<String>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            host: "0.0.0.0".into(),
            port: 8765,
            static_dir: None,
            tmp_dir: None,
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
pub struct AppConfig {
    pub server: ServerConfig,
    pub storages: Vec<StorageEntry>,
    pub adapters: AdaptersConfig,
    #[serde(default)]
    pub external_media_roots: Vec<PathBuf>,
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
            external_media_roots: Vec::new(),
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

    /// Return all allowed filesystem roots from which external zero-duplication
    /// attachments may be served safely via `/v1/attachments/{id}`.
    pub fn allowed_media_roots(&self) -> Vec<PathBuf> {
        let mut roots = Vec::new();
        for p in &self.external_media_roots {
            if !p.as_os_str().is_empty() {
                roots.push(expand_tilde(p));
            }
        }
        if let Some(ref uback) = self.adapters.uback {
            if !uback.backup_root.as_os_str().is_empty() {
                roots.push(expand_tilde(&uback.backup_root));
            }
        }
        if let Ok(env_root) = std::env::var("MYMESSAGES_UBACK_ROOT") {
            if !env_root.trim().is_empty() {
                roots.push(expand_tilde(Path::new(env_root.trim())));
            }
        }
        roots
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

#[path = "config_test.rs"]
#[cfg(test)]
mod tests;
