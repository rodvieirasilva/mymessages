use std::sync::{Arc, Mutex};
use anyhow::Result;

use crate::config::{AppConfig, StorageType};
use crate::index::Database;
use crate::models::CanonicalMessage;
use crate::storage::sink::{MaildirSink, SqliteSink, StorageSink};
use crate::storage::MaildirStorage;

#[derive(Default)]
pub struct StorageReport {
    pub eml_path: Option<String>,
    pub db_indexed: bool,
}

pub struct StoragePipeline {
    sinks: Vec<Arc<dyn StorageSink>>,
}

impl StoragePipeline {
    pub fn new() -> Self {
        Self { sinks: Vec::new() }
    }

    pub fn add_sink(&mut self, sink: Arc<dyn StorageSink>) {
        self.sinks.push(sink);
    }

    pub fn from_config(config: &AppConfig, db: Arc<Mutex<Database>>, default_storage: Arc<MaildirStorage>) -> Self {
        let mut pipeline = Self::new();
        for entry in &config.storages {
            match entry.storage_type {
                StorageType::Maildir => {
                    let storage = if entry.path == default_storage.root_dir {
                        default_storage.clone()
                    } else {
                        Arc::new(MaildirStorage::new(&entry.path))
                    };
                    pipeline.add_sink(Arc::new(MaildirSink::new(&entry.name, entry.enabled, storage)));
                }
                StorageType::Sqlite => {
                    pipeline.add_sink(Arc::new(SqliteSink::new(&entry.name, entry.enabled, db.clone())));
                }
            }
        }
        pipeline
    }

    pub fn write_all(&self, msg: &CanonicalMessage, content_hash: &str) -> Result<StorageReport> {
        let mut report = StorageReport::default();

        // 1. Run file sinks first (Maildir) to obtain file reference path
        for sink in &self.sinks {
            if sink.is_enabled() && (sink.name().contains("maildir") || sink.name().contains("mail")) {
                if let Some(res) = sink.write_message_with_path(msg, content_hash, "")? {
                    report.eml_path = Some(res);
                }
            }
        }

        // 2. Run index sinks (SQLite) providing the relative file path
        let file_path = report.eml_path.as_deref().unwrap_or("");
        for sink in &self.sinks {
            if sink.is_enabled() && !sink.name().contains("maildir") && !sink.name().contains("mail") {
                if let Some(_res) = sink.write_message_with_path(msg, content_hash, file_path)? {
                    report.db_indexed = true;
                }
            }
        }

        Ok(report)
    }

    pub fn list_sinks(&self) -> Vec<(&str, bool)> {
        self.sinks.iter().map(|s| (sink_name(s.as_ref()), s.is_enabled())).collect()
    }
}

fn sink_name(s: &dyn StorageSink) -> &str {
    s.name()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::StorageEntry;
    use crate::models::CanonicalParticipant;
    use chrono::Utc;

    #[test]
    fn test_storage_pipeline() {
        let temp_dir = std::env::temp_dir().join(format!("test_pipeline_{}", std::process::id()));
        let storage = Arc::new(MaildirStorage::new(&temp_dir));
        let db = Arc::new(Mutex::new(Database::in_memory().unwrap()));

        let mut cfg = AppConfig::default();
        cfg.storages = vec![
            StorageEntry::new_maildir("maildir", &temp_dir),
            StorageEntry::new_sqlite("sqlite", ":memory:"),
        ];
        let pipeline = StoragePipeline::from_config(&cfg, db, storage);

        assert_eq!(pipeline.list_sinks().len(), 2);

        let msg = CanonicalMessage {
            id: "pipe_1".into(),
            source: "whatsapp".into(),
            conversation_id: "c1".into(),
            conversation_name: None,
            is_group: false,
            sender: CanonicalParticipant { id: "u1".into(), name: None, phone_number: None },
            recipients: vec![],
            timestamp: Utc::now(),
            body: Some("Pipeline test".into()),
            attachments: vec![],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({}),
        };

        let report = pipeline.write_all(&msg, "hash999").unwrap();
        assert!(report.eml_path.is_some());
        assert!(report.db_indexed);

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
