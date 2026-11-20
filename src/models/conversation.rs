use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

pub fn scoped_id(source: &str, account: &str, item_id: &str) -> String {
    if account.is_empty() || account == "default" {
        format!("{}:{}", source, item_id)
    } else {
        format!("{}:{}:{}", source, account, item_id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub source_id: String,
    pub account_id: String,
    pub remote_id: String,
    pub name: Option<String>,
    pub is_group: bool,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl Conversation {
    pub fn new(source: &str, remote_id: &str, name: Option<String>, is_group: bool) -> Self {
        Self::for_account(source, "default", remote_id, name, is_group)
    }

    pub fn for_account(
        source: &str,
        account: &str,
        remote_id: &str,
        name: Option<String>,
        is_group: bool,
    ) -> Self {
        let acct = if account.is_empty() { "default" } else { account };
        Self {
            id: scoped_id(source, acct, remote_id),
            source_id: source.to_string(),
            account_id: acct.to_string(),
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
    pub account_id: String,
    pub remote_id: String,
    pub display_name: Option<String>,
    pub phone_number: Option<String>,
    pub avatar_url: Option<String>,
}

impl Contact {
    pub fn new(source: &str, remote_id: &str, display_name: Option<String>, phone: Option<String>) -> Self {
        Self::for_account(source, "default", remote_id, display_name, phone)
    }

    pub fn for_account(
        source: &str,
        account: &str,
        remote_id: &str,
        display_name: Option<String>,
        phone: Option<String>,
    ) -> Self {
        let acct = if account.is_empty() { "default" } else { account };
        Self {
            id: scoped_id(source, acct, remote_id),
            source_id: source.to_string(),
            account_id: acct.to_string(),
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
        assert_eq!(conv.account_id, "default");
        assert_eq!(conv.name.as_deref(), Some("Alice"));
        assert!(!conv.is_group);

        let conv2 = Conversation::for_account("whatsapp", "acct_a", "12345@s.whatsapp.net", Some("Alice".into()), false);
        assert_eq!(conv2.id, "whatsapp:acct_a:12345@s.whatsapp.net");
        assert_eq!(conv2.account_id, "acct_a");
    }

    #[test]
    fn test_contact_creation() {
        let contact = Contact::new("whatsapp", "12345", Some("Bob".into()), Some("+123456789".into()));
        assert_eq!(contact.id, "whatsapp:12345");
        assert_eq!(contact.account_id, "default");
        assert_eq!(contact.display_name.as_deref(), Some("Bob"));
        assert_eq!(contact.phone_number.as_deref(), Some("+123456789"));

        let contact2 = Contact::for_account("whatsapp", "acct_b", "12345", Some("Bob".into()), None);
        assert_eq!(contact2.id, "whatsapp:acct_b:12345");
        assert_eq!(contact2.account_id, "acct_b");
    }
}
