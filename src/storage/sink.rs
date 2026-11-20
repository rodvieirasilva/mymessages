use std::sync::{Arc, Mutex};
use anyhow::Result;

use crate::index::{index_message, Database};
use crate::models::CanonicalMessage;
use crate::storage::{build_eml, MaildirStorage};

pub trait StorageSink: Send + Sync {
    fn name(&self) -> &str;
    fn is_enabled(&self) -> bool;
    fn write_message_with_path(&self, msg: &CanonicalMessage, content_hash: &str, path: &str) -> Result<Option<String>>;
}

pub struct MaildirSink {
    name: String,
    enabled: bool,
    storage: Arc<MaildirStorage>,
}

impl MaildirSink {
    pub fn new(name: &str, enabled: bool, storage: Arc<MaildirStorage>) -> Self {
        Self {
            name: name.to_string(),
            enabled,
            storage,
        }
    }
}

impl StorageSink for MaildirSink {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn write_message_with_path(&self, msg: &CanonicalMessage, content_hash: &str, _path: &str) -> Result<Option<String>> {
        if !self.enabled {
            return Ok(None);
        }
        let eml_content = build_eml(msg, content_hash);
        let rel_path = self.storage.write_message_for_account(
            &msg.source,
            &msg.account,
            &msg.conversation_id,
            &msg.id,
            msg.timestamp.timestamp(),
            content_hash,
            &eml_content,
        )?;
        Ok(Some(rel_path.to_string_lossy().to_string()))
    }
}

pub struct SqliteSink {
    name: String,
    enabled: bool,
    db: Arc<Mutex<Database>>,
}

impl SqliteSink {
    pub fn new(name: &str, enabled: bool, db: Arc<Mutex<Database>>) -> Self {
        Self {
            name: name.to_string(),
            enabled,
            db,
        }
    }
}

impl StorageSink for SqliteSink {
    fn name(&self) -> &str {
        &self.name
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn write_message_with_path(&self, msg: &CanonicalMessage, content_hash: &str, path: &str) -> Result<Option<String>> {
        if !self.enabled {
            return Ok(None);
        }
        let mut db = self.db.lock().map_err(|e| anyhow::anyhow!("DB lock error: {}", e))?;
        index_message(db.conn_mut(), msg, content_hash, path)?;
        Ok(Some(msg.id.clone()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::CanonicalParticipant;
    use chrono::Utc;

    #[test]
    fn test_maildir_and_sqlite_sinks() {
        let temp_dir = std::env::temp_dir().join(format!("test_sinks_{}", std::process::id()));
        let storage = Arc::new(MaildirStorage::new(&temp_dir));
        let maildir_sink = MaildirSink::new("my_maildir", true, storage);

        let db = Arc::new(Mutex::new(Database::in_memory().unwrap()));
        let sqlite_sink = SqliteSink::new("my_sqlite", true, db);

        let msg = CanonicalMessage {
            id: "sink_msg_1".into(),
            source: "whatsapp".into(),
            account: "default".into(),
            conversation_id: "chat_1".into(),
            conversation_name: None,
            is_group: false,
            sender: CanonicalParticipant { id: "u1".into(), name: None, phone_number: None },
            recipients: vec![],
            timestamp: Utc::now(),
            body: Some("Testing sink".into()),
            attachments: vec![],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({}),
        };

        assert_eq!(maildir_sink.name(), "my_maildir");
        assert!(maildir_sink.is_enabled());

        let res_mail = maildir_sink.write_message_with_path(&msg, "hash123", "").unwrap();
        assert!(res_mail.is_some());

        let eml_path = res_mail.unwrap();
        let res_sql = sqlite_sink.write_message_with_path(&msg, "hash123", &eml_path).unwrap();
        assert_eq!(res_sql, Some("sink_msg_1".into()));

        let _ = std::fs::remove_dir_all(temp_dir);
    }
}
