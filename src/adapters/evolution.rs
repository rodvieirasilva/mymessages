use anyhow::Result;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use serde_json::Value;

use crate::adapters::SourceAdapter;
use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};

pub struct EvolutionApiAdapter;

impl EvolutionApiAdapter {
    pub fn new() -> Self {
        Self
    }

    fn extract_media(msg_val: &Value, key: &str, default_mime: &str) -> Option<CanonicalAttachment> {
        let media_obj = msg_val.get(key)?;
        let mime = media_obj["mimetype"]
            .as_str()
            .unwrap_or(default_mime)
            .to_string();
        let fname = media_obj["fileName"]
            .as_str()
            .map(|s| s.to_string());

        let raw_b64 = media_obj["base64"]
            .as_str()
            .or_else(|| msg_val["base64"].as_str())
            .unwrap_or("");

        let clean_b64 = if let Some(idx) = raw_b64.find(";base64,") {
            &raw_b64[idx + 8..]
        } else {
            raw_b64
        };

        let data = BASE64.decode(clean_b64.trim()).unwrap_or_default();
        let mut att = CanonicalAttachment::new(fname, mime, data);

        if let Some(secs) = media_obj["seconds"].as_u64() {
            att = att.with_duration(secs as u32);
        }

        Some(att)
    }
}

impl Default for EvolutionApiAdapter {
    fn default() -> Self {
        Self::new()
    }
}

impl SourceAdapter for EvolutionApiAdapter {
    fn source_id(&self) -> &'static str {
        "whatsapp"
    }

    fn parse_webhook(&self, payload: &Value) -> Result<Option<CanonicalMessage>> {
        let data = if payload.get("event").and_then(|e| e.as_str()) == Some("messages.upsert") {
            payload.get("data")
        } else if payload.get("data").is_some() {
            payload.get("data")
        } else {
            Some(payload)
        };

        let data = match data {
            Some(d) => d,
            None => return Ok(None),
        };

        let key = match data.get("key") {
            Some(k) => k,
            None => return Ok(None),
        };

        let message_id = match key["id"].as_str() {
            Some(id) if !id.is_empty() => id.to_string(),
            _ => return Ok(None),
        };

        let remote_jid = match key["remoteJid"].as_str() {
            Some(jid) if !jid.is_empty() => jid.to_string(),
            _ => return Ok(None),
        };

        let is_from_me = key["fromMe"].as_bool().unwrap_or(false);
        let is_group = remote_jid.ends_with("@g.us");

        let sender_id = if is_from_me {
            "me".to_string()
        } else if is_group {
            key["participant"]
                .as_str()
                .or_else(|| data["participant"].as_str())
                .unwrap_or(&remote_jid)
                .to_string()
        } else {
            remote_jid.clone()
        };

        let sender_name = data["pushName"]
            .as_str()
            .map(|s| s.to_string())
            .or_else(|| if is_from_me { Some("Me".into()) } else { None });

        let ts_sec = data["messageTimestamp"].as_i64().unwrap_or_else(|| Utc::now().timestamp());
        let timestamp = DateTime::from_timestamp(ts_sec, 0).unwrap_or_else(Utc::now);

        let msg_val = data.get("message").unwrap_or(&Value::Null);

        let mut body = msg_val["conversation"]
            .as_str()
            .or_else(|| msg_val["extendedTextMessage"]["text"].as_str())
            .map(|s| s.to_string());

        let mut attachments = Vec::new();

        if let Some(att) = Self::extract_media(msg_val, "imageMessage", "image/jpeg") {
            if body.is_none() {
                body = msg_val["imageMessage"]["caption"].as_str().map(|s| s.to_string());
            }
            attachments.push(att);
        } else if let Some(att) = Self::extract_media(msg_val, "audioMessage", "audio/ogg") {
            attachments.push(att);
        } else if let Some(att) = Self::extract_media(msg_val, "videoMessage", "video/mp4") {
            if body.is_none() {
                body = msg_val["videoMessage"]["caption"].as_str().map(|s| s.to_string());
            }
            attachments.push(att);
        } else if let Some(att) = Self::extract_media(msg_val, "documentMessage", "application/octet-stream") {
            if body.is_none() {
                body = msg_val["documentMessage"]["caption"].as_str().map(|s| s.to_string());
            }
            attachments.push(att);
        } else if let Some(att) = Self::extract_media(msg_val, "stickerMessage", "image/webp") {
            attachments.push(att);
        }

        let reply_to_id = msg_val["extendedTextMessage"]["contextInfo"]["stanzaId"]
            .as_str()
            .map(|s| s.to_string());

        let conversation_name = if is_group {
            data["chat"]["name"].as_str().map(|s| s.to_string())
        } else {
            sender_name.clone()
        };

        Ok(Some(CanonicalMessage {
            id: message_id,
            source: "whatsapp".to_string(),
            conversation_id: remote_jid,
            conversation_name,
            is_group,
            sender: CanonicalParticipant {
                id: sender_id,
                name: sender_name,
                phone_number: None,
            },
            recipients: Vec::new(),
            timestamp,
            body,
            attachments,
            reply_to_id,
            is_from_me,
            raw_payload: payload.clone(),
        }))
    }
}

#[path = "evolution_test.rs"]
#[cfg(test)]
mod tests;
