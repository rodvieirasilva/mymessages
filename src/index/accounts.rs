use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AccountSummary {
    pub id: String,
    pub name: String,
    pub sources: Vec<String>,
    pub message_count: i64,
    pub chat_count: i64,
}

pub fn list_accounts(conn: &Connection) -> Result<Vec<AccountSummary>> {
    let sql = "
        SELECT
            a.id,
            a.name,
            COALESCE((SELECT GROUP_CONCAT(DISTINCT m.source_id) FROM messages m WHERE m.account_id = a.id), '') AS sources_csv,
            (SELECT count(*) FROM messages m WHERE m.account_id = a.id) AS msg_count,
            (SELECT count(*) FROM conversations c WHERE c.account_id = a.id) AS chat_count
        FROM accounts a
        ORDER BY msg_count DESC, a.id ASC
    ";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([], |row| {
        let sources_csv: String = row.get(2)?;
        let sources: Vec<String> = sources_csv
            .split(',')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();
        Ok(AccountSummary {
            id: row.get(0)?,
            name: row.get(1)?,
            sources,
            message_count: row.get(3)?,
            chat_count: row.get(4)?,
        })
    })?;

    let mut accounts = Vec::new();
    for row in rows {
        accounts.push(row?);
    }
    Ok(accounts)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::{index_message, Database};
    use crate::models::{CanonicalMessage, CanonicalParticipant};
    use chrono::Utc;

    #[test]
    fn test_list_accounts_cross_account() {
        let mut db = Database::in_memory().unwrap();
        for (acct, mid) in [("alice_phone", "m1"), ("bob_phone", "m2"), ("alice_phone", "m3")] {
            let msg = CanonicalMessage {
                id: mid.into(),
                source: "whatsapp".into(),
                account: acct.into(),
                conversation_id: "c1@s.whatsapp.net".into(),
                conversation_name: Some("Chat".into()),
                is_group: false,
                sender: CanonicalParticipant { id: "u1".into(), name: Some("User".into()), phone_number: None },
                recipients: vec![],
                timestamp: Utc::now(),
                body: Some("hello".into()),
                attachments: vec![],
                reply_to_id: None,
                is_from_me: false,
                raw_payload: serde_json::json!({}),
            };
            index_message(db.conn_mut(), &msg, &format!("h_{}", mid), "mail/test.eml").unwrap();
        }

        let accounts = list_accounts(db.conn()).unwrap();
        assert_eq!(accounts.len(), 2);
        assert_eq!(accounts[0].id, "alice_phone");
        assert_eq!(accounts[0].message_count, 2);
        assert_eq!(accounts[0].sources, vec!["whatsapp"]);
        assert_eq!(accounts[1].id, "bob_phone");
        assert_eq!(accounts[1].message_count, 1);
    }
}
