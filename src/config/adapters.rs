use std::path::PathBuf;
use serde::{Deserialize, Serialize};

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

fn default_poll_interval() -> u64 {
    30
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct UbackAdapterConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default)]
    pub backup_root: PathBuf,
    #[serde(default)]
    pub credentials_file: Option<PathBuf>,
    #[serde(default)]
    pub accounts: Vec<String>,
    #[serde(default)]
    pub index_media_without_db: bool,
    #[serde(default = "default_poll_interval")]
    pub poll_interval_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct AdaptersConfig {
    pub evolution: EvolutionAdapterConfig,
    pub waha: WahaAdapterConfig,
    pub folder: Option<FolderAdapterConfig>,
    #[serde(default)]
    pub uback: Option<UbackAdapterConfig>,
}
