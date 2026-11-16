use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub source_id: String,
    pub remote_id: String,
    pub name: Option<String>,
    pub is_group: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Conversation {
    pub fn new(source: &str, remote_id: &str, name: Option<String>, is_group: bool) -> Self {
        Self {
            id: format!("{}:{}", source, remote_id),
            source_id: source.to_string(),
            remote_id: remote_id.to_string(),
            name,
            is_group,
            created_at: Some(Utc::now()),
            updated_at: Some(Utc::now()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Contact {
    pub id: String,
    pub source_id: String,
    pub remote_id: String,
    pub display_name: Option<String>,
    pub phone_number: Option<String>,
    pub avatar_url: Option<String>,
}

impl Contact {
    pub fn new(source: &str, remote_id: &str, display_name: Option<String>, phone: Option<String>) -> Self {
        Self {
            id: format!("{}:{}", source, remote_id),
            source_id: source.to_string(),
            remote_id: remote_id.to_string(),
            display_name,
            phone_number: phone,
            avatar_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_conversation_creation() {
        let conv = Conversation::new("whatsapp", "12345@s.whatsapp.net", Some("Alice".into()), false);
        assert_eq!(conv.id, "whatsapp:12345@s.whatsapp.net");
        assert_eq!(conv.source_id, "whatsapp");
        assert_eq!(conv.name.as_deref(), Some("Alice"));
        assert!(!conv.is_group);
    }

    #[test]
    fn test_contact_creation() {
        let contact = Contact::new("whatsapp", "12345", Some("Bob".into()), Some("+123456789".into()));
        assert_eq!(contact.id, "whatsapp:12345");
        assert_eq!(contact.display_name.as_deref(), Some("Bob"));
        assert_eq!(contact.phone_number.as_deref(), Some("+123456789"));
    }
}
