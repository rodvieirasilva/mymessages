use anyhow::{Context, Result};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde_json::Value;

use super::SourceAdapter;
use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};

pub struct WahaAdapter;

impl WahaAdapter {
    pub fn new() -> Self {
        Self
    }

    fn parse_media(media_val: &Value) -> Option<CanonicalAttachment> {
        let mimetype = media_val.get("mimetype")?.as_str()?.to_string();
        let filename = media_val.get("filename").and_then(|f| f.as_str()).map(|s| s.to_string());

        let data = if let Some(b64) = media_val.get("data").and_then(|d| d.as_str()) {
            let clean_b64 = if let Some(idx) = b64.find(";base64,") {
                &b64[idx + 8..]
            } else {
                b64
            };
            BASE64.decode(clean_b64).unwrap_or_default()
        } else {
            Vec::new()
        };

        Some(CanonicalAttachment::new(filename, mimetype, data))
    }
}

impl Default for WahaAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceAdapter for WahaAdapter {
    fn source_id(&self) -> &'static str {
        "whatsapp"
    }

    fn parse_webhook(&self, payload: &Value) -> Result<Option<CanonicalMessage>> {
        let event = payload.get("event").and_then(|e| e.as_str()).unwrap_or("");
        if event != "message" && event != "message.any" {
            return Ok(None);
        }

        let p = payload.get("payload").context("Missing WAHA payload object")?;
        let msg_id = p.get("id").and_then(|id| id.as_str()).context("Missing message id")?.to_string();

        let ts_sec = p.get("timestamp").and_then(|t| t.as_i64()).unwrap_or_else(|| Utc::now().timestamp());
        let timestamp = DateTime::from_timestamp(ts_sec, 0).unwrap_or_else(Utc::now);

        let from_me = p.get("fromMe").and_then(|f| f.as_bool()).unwrap_or(false);
        let from_jid = p.get("from").and_then(|f| f.as_str()).unwrap_or("unknown@c.us");
        let to_jid = p.get("to").and_then(|t| t.as_str()).unwrap_or("unknown@c.us");

        let is_group = from_jid.ends_with("@g.us") || to_jid.ends_with("@g.us");
        let conversation_id = if is_group {
            if from_jid.ends_with("@g.us") { from_jid.to_string() } else { to_jid.to_string() }
        } else if from_me {
            to_jid.to_string()
        } else {
            from_jid.to_string()
        };

        let notify_name = p.get("_data")
            .and_then(|d| d.get("notifyName"))
            .and_then(|n| n.as_str());

        let sender_id = if is_group {
            p.get("_data")
                .and_then(|d| d.get("author"))
                .and_then(|a| a.as_str())
                .unwrap_or(from_jid)
                .to_string()
        } else {
            from_jid.to_string()
        };

        let sender = CanonicalParticipant {
            id: sender_id.clone(),
            name: notify_name.map(|n| n.to_string()),
            phone_number: Some(sender_id.split('@').next().unwrap_or(&sender_id).to_string()),
        };

        let body = p.get("body").and_then(|b| b.as_str()).map(|s| s.to_string());
        let reply_to_id = p.get("replyTo").and_then(|r| r.get("id")).and_then(|id| id.as_str()).map(|s| s.to_string());

        let mut attachments = Vec::new();
        let has_media = p.get("hasMedia").and_then(|h| h.as_bool()).unwrap_or(false);
        if has_media {
            if let Some(media_val) = p.get("media") {
                if let Some(att) = Self::parse_media(media_val) {
                    attachments.push(att);
                }
            }
        }

        let conv_name = if is_group {
            Some(format!("Group ({})", conversation_id.split('@').next().unwrap_or(&conversation_id)))
        } else {
            notify_name.map(|n| n.to_string())
        };

        Ok(Some(CanonicalMessage {
            id: msg_id,
            source: self.source_id().to_string(),
            conversation_id,
            conversation_name: conv_name,
            sender,
            recipients: Vec::new(),
            timestamp,
            body,
            attachments,
            reply_to_id,
            is_group,
            is_from_me: from_me,
            raw_payload: payload.clone(),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::MediaType;
    use serde_json::json;

    #[test]
    fn test_waha_parse_text_message() {
        let adapter = WahaAdapter::new();
        let payload = json!({
            "event": "message",
            "session": "default",
            "payload": {
                "id": "false_5511999998888@c.us_WAHA001",
                "timestamp": 1747707858,
                "from": "5511999998888@c.us",
                "to": "5511988887777@c.us",
                "fromMe": false,
                "body": "Hello from WAHA!",
                "hasMedia": false,
                "_data": {
                    "notifyName": "Bob"
                }
            }
        });

        let msg = adapter.parse_webhook(&payload).unwrap().unwrap();
        assert_eq!(msg.id, "false_5511999998888@c.us_WAHA001");
        assert_eq!(msg.source, "whatsapp");
        assert_eq!(msg.conversation_id, "5511999998888@c.us");
        assert_eq!(msg.sender.name.as_deref(), Some("Bob"));
        assert_eq!(msg.body.as_deref(), Some("Hello from WAHA!"));
        assert!(!msg.is_group);
        assert!(!msg.is_from_me);
    }

    #[test]
    fn test_waha_parse_image_media() {
        let adapter = WahaAdapter::new();
        let payload = json!({
            "event": "message.any",
            "payload": {
                "id": "true_120363024@g.us_WAHA002",
                "timestamp": 1747707900,
                "from": "120363024@g.us",
                "to": "5511988887777@c.us",
                "fromMe": true,
                "body": "Check out this screenshot",
                "hasMedia": true,
                "media": {
                    "mimetype": "image/png",
                    "filename": "screenshot.png",
                    "data": "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNk+A8AAQUBAScY42YAAAAASUVORK5CYII="
                },
                "_data": {
                    "author": "5511988887777@c.us"
                }
            }
        });

        let msg = adapter.parse_webhook(&payload).unwrap().unwrap();
        assert_eq!(msg.id, "true_120363024@g.us_WAHA002");
        assert!(msg.is_group);
        assert!(msg.is_from_me);
        assert_eq!(msg.attachments.len(), 1);
        assert_eq!(msg.attachments[0].filename.as_deref(), Some("screenshot.png"));
        assert_eq!(msg.attachments[0].media_type, MediaType::Image);
        assert!(!msg.attachments[0].data.is_empty());
    }
}
