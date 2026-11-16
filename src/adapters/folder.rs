use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::adapters::SourceAdapter;
use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};

pub struct FolderAdapter {
    folder_path: PathBuf,
}

impl FolderAdapter {
    pub fn new<P: AsRef<Path>>(folder_path: P) -> Self {
        Self {
            folder_path: folder_path.as_ref().to_path_buf(),
        }
    }

    pub fn parse_message_json(val: &Value) -> Result<CanonicalMessage> {
        let id = val["id"].as_str().context("Missing id in message json")?.to_string();
        let source = val["source"].as_str().unwrap_or("folder").to_string();
        let conversation_id = val["conversation_id"].as_str().context("Missing conversation_id")?.to_string();
        let conversation_name = val["conversation_name"].as_str().map(|s| s.to_string());
        let is_group = val["is_group"].as_bool().unwrap_or(false);

        let sender_obj = &val["sender"];
        let sender = CanonicalParticipant {
            id: sender_obj["id"].as_str().unwrap_or("unknown").to_string(),
            name: sender_obj["name"].as_str().map(|s| s.to_string()),
            phone_number: sender_obj["phone_number"].as_str().map(|s| s.to_string()),
        };

        let ts_sec = val["timestamp"].as_i64().unwrap_or_else(|| Utc::now().timestamp());
        let timestamp = DateTime::from_timestamp(ts_sec, 0).unwrap_or_else(Utc::now);
        let body = val["body"].as_str().map(|s| s.to_string());
        let is_from_me = val["is_from_me"].as_bool().unwrap_or(false);

        let mut attachments = Vec::new();
        if let Some(att_list) = val["attachments"].as_array() {
            for item in att_list {
                let fname = item["filename"].as_str().map(|s| s.to_string());
                let mime = item["mimetype"].as_str().unwrap_or("application/octet-stream").to_string();
                let b64 = item["base64"].as_str().unwrap_or("");
                let data = BASE64.decode(b64.trim()).unwrap_or_default();
                let mut att = CanonicalAttachment::new(fname, mime, data);
                if let Some(secs) = item["duration_seconds"].as_u64() {
                    att = att.with_duration(secs as u32);
                }
                attachments.push(att);
            }
        }

        Ok(CanonicalMessage {
            id,
            source,
            conversation_id,
            conversation_name,
            is_group,
            sender,
            recipients: Vec::new(),
            timestamp,
            body,
            attachments,
            reply_to_id: None,
            is_from_me,
            raw_payload: val.clone(),
        })
    }

    pub fn read_all_messages(&self) -> Result<Vec<CanonicalMessage>> {
        let mut messages = Vec::new();
        if !self.folder_path.exists() {
            return Ok(messages);
        }

        let mut entries: Vec<_> = fs::read_dir(&self.folder_path)?
            .filter_map(|r| r.ok())
            .filter(|e| e.path().extension().and_then(|ext| ext.to_str()) == Some("json"))
            .collect();
        entries.sort_by_key(|e| e.path());

        for entry in entries {
            let content = fs::read_to_string(entry.path())?;
            let val: Value = serde_json::from_str(&content)?;
            if let Ok(msg) = Self::parse_message_json(&val) {
                messages.push(msg);
            }
        }

        Ok(messages)
    }
}

impl SourceAdapter for FolderAdapter {
    fn source_id(&self) -> &'static str {
        "folder"
    }

    fn parse_webhook(&self, payload: &Value) -> Result<Option<CanonicalMessage>> {
        Self::parse_message_json(payload).map(Some)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_folder_adapter_read_test_messages() {
        let adapter = FolderAdapter::new("./tests/test_messages");
        let messages = adapter.read_all_messages().unwrap();
        assert!(messages.len() >= 4);

        let msg1 = &messages[0];
        assert_eq!(msg1.id, "MSG_FOLDER_001");
        assert_eq!(msg1.conversation_name.as_deref(), Some("Team Alpha"));
        assert!(msg1.is_group);

        let msg2 = &messages[1];
        assert_eq!(msg2.attachments.len(), 1);
        assert_eq!(msg2.attachments[0].filename.as_deref(), Some("architecture_diagram.png"));
    }
}
