use anyhow::Result;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::index::fts::{get_message_attachments, SearchResult};
use crate::index::lucene::parse_lucene_string;
use crate::index::query_dsl::CompiledQuery;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
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

pub fn list_chats(
    conn: &Connection,
    source_filter: Option<&str>,
    query_filter: Option<&str>,
) -> Result<Vec<ChatSummary>> {
    let clean_query = query_filter.unwrap_or("").trim();
    let (effective_source, query_term) = if clean_query.is_empty() {
        (source_filter.map(|s| s.to_string()), None)
    } else {
        let mut compiled = CompiledQuery::default();
        parse_lucene_string(clean_query, &mut compiled);
        let src = source_filter.map(|s| s.to_string()).or(compiled.source);
        let term = if !compiled.text_terms.is_empty() {
            Some(compiled.text_terms.join(" "))
        } else if let Some(sender) = compiled.sender {
            Some(sender)
        } else if clean_query.contains(':') {
            None
        } else {
            Some(clean_query.to_string())
        };
        (src, term)
    };

    let sql = "
        SELECT 
            c.id, c.source_id, c.remote_id, c.name, c.is_group,
            count(m.id) as msg_count,
            (SELECT body FROM messages WHERE conversation_id = c.id ORDER BY timestamp DESC LIMIT 1) as last_msg,
            max(m.timestamp) as last_ts
        FROM conversations c
        LEFT JOIN messages m ON m.conversation_id = c.id
        WHERE (?1 IS NULL OR c.source_id = ?1)
          AND (?2 IS NULL OR (
              c.name LIKE '%' || ?2 || '%' 
              OR c.remote_id LIKE '%' || ?2 || '%' 
              OR c.id IN (
                  SELECT DISTINCT m2.conversation_id 
                  FROM messages m2 
                  LEFT JOIN contacts ct ON ct.id = m2.sender_id 
                  WHERE m2.body LIKE '%' || ?2 || '%' OR ct.display_name LIKE '%' || ?2 || '%'
              )
          ))
        GROUP BY c.id
        ORDER BY last_ts DESC NULLS LAST
    ";

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map([effective_source.as_deref(), query_term.as_deref()], |row| {
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
