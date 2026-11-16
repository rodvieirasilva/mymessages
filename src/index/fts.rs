use anyhow::{Context, Result};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::index::query_dsl::CompiledQuery;

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct SearchQuery {
    #[serde(default)]
    pub query: String,
    pub source: Option<String>,
    pub chat_id: Option<String>,
    pub has_media: Option<bool>,
    pub media_type: Option<String>,
    pub is_from_me: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl SearchQuery {
    pub fn to_compiled(&self) -> CompiledQuery {
        let mut compiled = CompiledQuery {
            source: self.source.clone(),
            chat_id: self.chat_id.clone(),
            has_media: self.has_media,
            media_type: self.media_type.clone(),
            is_from_me: self.is_from_me,
            limit: self.limit.unwrap_or(50).clamp(1, 100),
            offset: self.offset.unwrap_or(0),
            ..Default::default()
        };
        crate::index::lucene::parse_lucene_string(&self.query, &mut compiled);
        compiled
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttachmentInfo {
    pub id: String,
    pub filename: Option<String>,
    pub mimetype: String,
    pub file_size: i64,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

pub fn search_messages(conn: &Connection, q: &SearchQuery) -> Result<Vec<SearchResult>> {
    let compiled = q.to_compiled();
    search_compiled(conn, &compiled)
}

pub fn search_compiled(conn: &Connection, q: &CompiledQuery) -> Result<Vec<SearchResult>> {
    let clean_query = q.text_terms.join(" ").trim().replace('"', "\"\"");
    let has_text = !clean_query.is_empty();
    let match_expr = if has_text {
        format!("\"{}\"", clean_query)
    } else {
        String::new()
    };

    let sender_pattern = q.sender.as_ref().map(|s| format!("%{}%", s));
    let order_clause = if q.sort_asc { "ORDER BY m.timestamp ASC" } else { "ORDER BY m.timestamp DESC" };

    let mut sql = if has_text {
        "SELECT 
            m.id, m.source_id, m.conversation_id, c.name, ct.display_name,
            m.timestamp, m.body, m.eml_path, m.has_attachments, m.is_from_me,
            snippet(messages_fts, 3, '<b>', '</b>', '...', 15) as snippet
        FROM messages_fts f
        JOIN messages m ON m.id = f.message_id
        JOIN conversations c ON c.id = m.conversation_id
        JOIN contacts ct ON ct.id = m.sender_id
        WHERE messages_fts MATCH ?1
          AND (?2 IS NULL OR m.source_id = ?2)
          AND (?3 IS NULL OR m.conversation_id = ?3 OR c.remote_id = ?3)
          AND (?4 IS NULL OR m.has_attachments = ?4)
          AND (?5 IS NULL OR m.is_from_me = ?5)
          AND (?6 IS NULL OR EXISTS (SELECT 1 FROM attachments a WHERE a.message_id = m.id AND a.media_type = ?6))
          AND (?7 IS NULL OR ct.display_name LIKE ?7 OR ct.remote_id LIKE ?7)
          AND (?8 IS NULL OR m.timestamp >= ?8)
          AND (?9 IS NULL OR m.timestamp <= ?9)".to_string()
    } else {
        "SELECT 
            m.id, m.source_id, m.conversation_id, c.name, ct.display_name,
            m.timestamp, m.body, m.eml_path, m.has_attachments, m.is_from_me,
            NULL as snippet
        FROM messages m
        JOIN conversations c ON c.id = m.conversation_id
        JOIN contacts ct ON ct.id = m.sender_id
        WHERE (?2 IS NULL OR m.source_id = ?2)
          AND (?3 IS NULL OR m.conversation_id = ?3 OR c.remote_id = ?3)
          AND (?4 IS NULL OR m.has_attachments = ?4)
          AND (?5 IS NULL OR m.is_from_me = ?5)
          AND (?6 IS NULL OR EXISTS (SELECT 1 FROM attachments a WHERE a.message_id = m.id AND a.media_type = ?6))
          AND (?7 IS NULL OR ct.display_name LIKE ?7 OR ct.remote_id LIKE ?7)
          AND (?8 IS NULL OR m.timestamp >= ?8)
          AND (?9 IS NULL OR m.timestamp <= ?9)".to_string()
    };

    for (k, v) in &q.metadata_filters {
        let escaped_k = k.replace('\'', "''");
        let escaped_v = v.replace('\'', "''");
        sql.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM message_metadata md WHERE md.message_id = m.id AND md.key = '{}' AND md.value = '{}')",
            escaped_k, escaped_v
        ));
    }

    sql.push_str(&format!(" {} LIMIT ?10 OFFSET ?11", order_clause));

    let mut stmt = conn.prepare(&sql).context("Failed to prepare FTS search query")?;
    let rows = stmt.query_map(
        rusqlite::params![
            match_expr,
            q.source,
            q.chat_id,
            q.has_media,
            q.is_from_me,
            q.media_type,
            sender_pattern,
            q.from_timestamp,
            q.to_timestamp,
            q.limit as i64,
            q.offset as i64,
        ],
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

pub fn get_message_attachments(conn: &Connection, message_id: &str) -> Result<Vec<AttachmentInfo>> {
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
    fn test_search_and_metadata_filter() {
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
            raw_payload: serde_json::json!({ "session": "work", "priority": "high" }),
        };

        index_message(db.conn_mut(), &msg, "hash_fts_1", "mail/wh/cur/1.eml").unwrap();

        // 1. Basic query
        let q = SearchQuery {
            query: "sqlite".into(),
            ..Default::default()
        };
        let results = search_messages(db.conn(), &q).unwrap();
        assert_eq!(results.len(), 1);

        // 2. Lucene metadata query
        let q_meta = SearchQuery {
            query: "meta.session:work".into(),
            ..Default::default()
        };
        let res_meta = search_messages(db.conn(), &q_meta).unwrap();
        assert_eq!(res_meta.len(), 1);
    }
}
