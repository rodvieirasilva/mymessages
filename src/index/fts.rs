use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SearchQuery {
    pub query: String,
    pub source: Option<String>,
    pub chat_id: Option<String>,
    pub has_media: Option<bool>,
    pub media_type: Option<String>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AttachmentInfo {
    pub id: String,
    pub filename: Option<String>,
    pub mimetype: String,
    pub file_size: i64,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub id: String,
    pub source: String,
    pub conversation_id: String,
    pub conversation_name: Option<String>,
    pub sender_name: Option<String>,
    pub timestamp: i64,
    pub body: Option<String>,
    pub eml_path: String,
    pub has_attachments: bool,
    pub is_from_me: bool,
    pub snippet: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatSummary {
    pub id: String,
    pub source_id: String,
    pub remote_id: String,
    pub name: Option<String>,
    pub is_group: bool,
    pub message_count: i64,
    pub last_message: Option<String>,
    pub last_timestamp: Option<i64>,
}

pub fn search_messages(conn: &Connection, q: &SearchQuery) -> Result<Vec<SearchResult>> {
    let limit = q.limit.unwrap_or(50).min(100);
    let offset = q.offset.unwrap_or(0);

    let clean_query = q.query.trim().replace('"', "\"\"");
    let match_expr = if clean_query.is_empty() {
        "*".to_string()
    } else {
        format!("\"{}\"", clean_query)
    };

    let sql = "
        SELECT 
            m.id, m.source_id, m.conversation_id, c.name, ct.display_name,
            m.timestamp, m.body, m.eml_path, m.has_attachments, m.is_from_me,
            snippet(messages_fts, 3, '<b>', '</b>', '...', 15) as snippet
        FROM messages_fts f
        JOIN messages m ON m.id = f.message_id
        JOIN conversations c ON c.id = m.conversation_id
        JOIN contacts ct ON ct.id = m.sender_id
        WHERE messages_fts MATCH ?1
          AND (?2 IS NULL OR m.source_id = ?2)
          AND (?3 IS NULL OR m.conversation_id = ?3)
          AND (?4 IS NULL OR m.has_attachments = ?4)
        ORDER BY m.timestamp DESC
        LIMIT ?5 OFFSET ?6
    ";

    let mut stmt = conn.prepare(sql).context("Failed to prepare FTS search query")?;
    let rows = stmt.query_map(
        rusqlite::params![match_expr, q.source, q.chat_id, q.has_media, limit as i64, offset as i64],
        |row| {
            Ok(SearchResult {
                id: row.get(0)?,
                source: row.get(1)?,
                conversation_id: row.get(2)?,
                conversation_name: row.get(3)?,
                sender_name: row.get(4)?,
                timestamp: row.get(5)?,
                body: row.get(6)?,
                eml_path: row.get(7)?,
                has_attachments: row.get(8)?,
                is_from_me: row.get(9)?,
                snippet: row.get(10)?,
                attachments: Vec::new(),
            })
        },
    )?;

    let mut results = Vec::new();
    for row in rows {
        let mut res = row?;
        if res.has_attachments {
            res.attachments = get_message_attachments(conn, &res.id)?;
        }
        results.push(res);
    }

    Ok(results)
}

pub fn list_chats(conn: &Connection, source_filter: Option<&str>) -> Result<Vec<ChatSummary>> {
    let sql = "
        SELECT 
            c.id, c.source_id, c.remote_id, c.name, c.is_group,
            count(m.id) as msg_count,
            (SELECT body FROM messages WHERE conversation_id = c.id ORDER BY timestamp DESC LIMIT 1) as last_msg,
            max(m.timestamp) as last_ts
        FROM conversations c
        LEFT JOIN messages m ON m.conversation_id = c.id
        WHERE (?1 IS NULL OR c.source_id = ?1)
        GROUP BY c.id
        ORDER BY last_ts DESC NULLS LAST
    ";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([source_filter], |row| {
        Ok(ChatSummary {
            id: row.get(0)?,
            source_id: row.get(1)?,
            remote_id: row.get(2)?,
            name: row.get(3)?,
            is_group: row.get(4)?,
            message_count: row.get(5)?,
            last_message: row.get(6)?,
            last_timestamp: row.get(7)?,
        })
    })?;

    let mut list = Vec::new();
    for row in rows {
        list.push(row?);
    }
    Ok(list)
}

pub fn get_chat_messages(
    conn: &Connection,
    chat_id: &str,
    limit: usize,
    before: Option<i64>,
) -> Result<Vec<SearchResult>> {
    let sql = "
        SELECT 
            m.id, m.source_id, m.conversation_id, c.name, ct.display_name,
            m.timestamp, m.body, m.eml_path, m.has_attachments, m.is_from_me
        FROM messages m
        JOIN conversations c ON c.id = m.conversation_id
        JOIN contacts ct ON ct.id = m.sender_id
        WHERE m.conversation_id = ?1
          AND (?2 IS NULL OR m.timestamp < ?2)
        ORDER BY m.timestamp DESC
        LIMIT ?3
    ";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(rusqlite::params![chat_id, before, limit as i64], |row| {
        Ok(SearchResult {
            id: row.get(0)?,
            source: row.get(1)?,
            conversation_id: row.get(2)?,
            conversation_name: row.get(3)?,
            sender_name: row.get(4)?,
            timestamp: row.get(5)?,
            body: row.get(6)?,
            eml_path: row.get(7)?,
            has_attachments: row.get(8)?,
            is_from_me: row.get(9)?,
            snippet: None,
            attachments: Vec::new(),
        })
    })?;

    let mut results = Vec::new();
    for row in rows {
        let mut res = row?;
        if res.has_attachments {
            res.attachments = get_message_attachments(conn, &res.id)?;
        }
        results.push(res);
    }
    Ok(results)
}

fn get_message_attachments(conn: &Connection, message_id: &str) -> Result<Vec<AttachmentInfo>> {
    let mut stmt = conn.prepare(
        "SELECT id, filename, mimetype, file_size, media_type FROM attachments WHERE message_id = ?1",
    )?;
    let rows = stmt.query_map([message_id], |row| {
        Ok(AttachmentInfo {
            id: row.get(0)?,
            filename: row.get(1)?,
            mimetype: row.get(2)?,
            file_size: row.get(3)?,
            media_type: row.get(4)?,
        })
    })?;

    let mut list = Vec::new();
    for r in rows {
        list.push(r?);
    }
    Ok(list)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::db::Database;
    use crate::index::writer::index_message;
    use crate::models::{CanonicalMessage, CanonicalParticipant};
    use chrono::Utc;

    #[test]
    fn test_search_and_list_chats() {
        let mut db = Database::in_memory().unwrap();
        let msg = CanonicalMessage {
            id: "msg_fts_1".into(),
            source: "whatsapp".into(),
            conversation_id: "chat_abc".into(),
            conversation_name: Some("Tech Talk".into()),
            is_group: true,
            sender: CanonicalParticipant {
                id: "user_a".into(),
                name: Some("Alice".into()),
                phone_number: None,
            },
            recipients: vec![],
            timestamp: Utc::now(),
            body: Some("Searching for rust and sqlite".into()),
            attachments: vec![],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({}),
        };

        index_message(db.conn_mut(), &msg, "hash_fts_1", "mail/wh/cur/1.eml").unwrap();

        let q = SearchQuery {
            query: "sqlite".into(),
            ..Default::default()
        };
        let results = search_messages(db.conn(), &q).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].conversation_name.as_deref(), Some("Tech Talk"));

        let chats = list_chats(db.conn(), None).unwrap();
        assert_eq!(chats.len(), 1);
        assert_eq!(chats[0].message_count, 1);

        let timeline = get_chat_messages(db.conn(), "whatsapp:chat_abc", 10, None).unwrap();
        assert_eq!(timeline.len(), 1);
    }
}
