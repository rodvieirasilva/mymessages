use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use super::attachment::CanonicalAttachment;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalParticipant {
    pub id: String,
    pub name: Option<String>,
    pub phone_number: Option<String>,
}

pub fn default_account() -> String {
    "default".to_string()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CanonicalMessage {
    pub id: String,
    pub source: String,
    #[serde(default = "default_account")]
    pub account: String,
    pub conversation_id: String,
    pub conversation_name: Option<String>,
    pub is_group: bool,
    pub sender: CanonicalParticipant,
    pub recipients: Vec<CanonicalParticipant>,
    pub timestamp: DateTime<Utc>,
    pub body: Option<String>,
    pub attachments: Vec<CanonicalAttachment>,
    pub reply_to_id: Option<String>,
    pub is_from_me: bool,
    pub raw_payload: serde_json::Value,
}

impl CanonicalMessage {
    pub fn compute_content_hash(&self) -> String {
        let mut hasher = Sha256::new();
        hasher.update(self.source.as_bytes());
        hasher.update(b":");
        if self.account != "default" && !self.account.is_empty() {
            hasher.update(self.account.as_bytes());
            hasher.update(b":");
        }
        hasher.update(self.conversation_id.as_bytes());
        hasher.update(b":");
        hasher.update(self.id.as_bytes());
        hasher.update(b":");
        hasher.update(self.timestamp.timestamp().to_string().as_bytes());
        hasher.update(b":");
        if let Some(ref body) = self.body {
            hasher.update(body.as_bytes());
        }
        for att in &self.attachments {
            hasher.update(b":att:");
            hasher.update(att.sha256.as_bytes());
        }
        hex::encode(hasher.finalize())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_message_hash_deterministic() {
        let msg1 = CanonicalMessage {
            id: "msg_123".into(),
            source: "whatsapp".into(),
            account: "default".into(),
            conversation_id: "conv_456".into(),
            conversation_name: Some("Chat".into()),
            is_group: false,
            sender: CanonicalParticipant {
                id: "sender_1".into(),
                name: Some("Alice".into()),
                phone_number: None,
            },
            recipients: vec![],
            timestamp: DateTime::from_timestamp(1700000000, 0).unwrap(),
            body: Some("Hello world".into()),
            attachments: vec![],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({}),
        };

        let msg2 = msg1.clone();
        assert_eq!(msg1.compute_content_hash(), msg2.compute_content_hash());

        let mut msg3 = msg1.clone();
        msg3.body = Some("Different body".into());
        assert_ne!(msg1.compute_content_hash(), msg3.compute_content_hash());

        let mut msg4 = msg1.clone();
        msg4.account = "acct_b".into();
        assert_ne!(msg1.compute_content_hash(), msg4.compute_content_hash());
    }
}
