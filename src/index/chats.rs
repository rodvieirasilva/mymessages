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
    pub account_id: String,
    pub remote_id: String,
    pub name: Option<String>,
    pub is_group: bool,
    pub message_count: i64,
    pub last_message: Option<String>,
    pub last_timestamp: Option<i64>,
}

fn to_like_pattern(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.contains('*') {
        trimmed.replace('*', "%")
    } else {
        format!("%{}%", trimmed)
    }
}

pub fn list_chats(
    conn: &Connection,
    source_filter: Option<&str>,
    account_filter: Option<&str>,
    query_filter: Option<&str>,
    has_media: Option<bool>,
    media_type: Option<&str>,
    is_from_me: Option<bool>,
) -> Result<Vec<ChatSummary>> {
    let clean_query = query_filter.unwrap_or("").trim();
    let (effective_source, effective_account, query_term) = if clean_query.is_empty() {
        (
            source_filter.map(|s| s.to_string()),
            account_filter.map(|s| s.to_string()),
            None,
        )
    } else {
        let mut compiled = CompiledQuery::default();
        parse_lucene_string(clean_query, &mut compiled);
        let src = source_filter.map(|s| s.to_string()).or(compiled.source);
        let acct = account_filter.map(|s| s.to_string()).or(compiled.account);
        let term = if !compiled.text_terms.is_empty() {
            Some(compiled.text_terms.join(" "))
        } else if let Some(sender) = compiled.sender {
            Some(sender)
        } else if clean_query.contains(':') {
            None
        } else {
            Some(clean_query.to_string())
        };
        (src, acct, term)
    };

    let sql = "
        SELECT 
            c.id, c.source_id, c.account_id, c.remote_id, c.name, c.is_group,
            count(m.id) as msg_count,
            (SELECT body FROM messages WHERE conversation_id = c.id ORDER BY timestamp DESC LIMIT 1) as last_msg,
            max(m.timestamp) as last_ts
        FROM conversations c
        LEFT JOIN messages m ON m.conversation_id = c.id
        WHERE (?1 IS NULL OR c.source_id LIKE ?1)
          AND (?2 IS NULL OR c.account_id LIKE ?2)
          AND (?3 IS NULL OR (
              c.name LIKE ?3
              OR c.remote_id LIKE ?3
          ))
          AND (?4 IS NULL AND ?5 IS NULL AND ?6 IS NULL OR EXISTS (
              SELECT 1 FROM messages m3
              WHERE m3.conversation_id = c.id
                AND (?4 IS NULL OR m3.has_attachments = ?4)
                AND (?5 IS NULL OR EXISTS (SELECT 1 FROM attachments a WHERE a.message_id = m3.id AND a.media_type LIKE ?5))
                AND (?6 IS NULL OR m3.is_from_me = ?6)
          ))
        GROUP BY c.id
        ORDER BY last_ts DESC NULLS LAST
    ";

    let src_pat = effective_source.as_deref().map(to_like_pattern);
    let acct_pat = effective_account.as_deref().map(to_like_pattern);
    let term_pat = query_term.as_deref().map(to_like_pattern);
    let media_pat = media_type.map(to_like_pattern);

    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(
        rusqlite::params![
            src_pat.as_deref(),
            acct_pat.as_deref(),
            term_pat.as_deref(),
            has_media,
            media_pat.as_deref(),
            is_from_me,
        ],
        |row| {
            Ok(ChatSummary {
                id: row.get(0)?,
                source_id: row.get(1)?,
                account_id: row.get(2)?,
                remote_id: row.get(3)?,
                name: row.get(4)?,
                is_group: row.get(5)?,
                message_count: row.get(6)?,
                last_message: row.get(7)?,
                last_timestamp: row.get(8)?,
            })
        },
    )?;

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
            m.id, m.source_id, m.account_id, m.conversation_id, c.name, ct.display_name,
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
            account: row.get(2)?,
            conversation_id: row.get(3)?,
            conversation_name: row.get(4)?,
            sender_name: row.get(5)?,
            timestamp: row.get(6)?,
            body: row.get(7)?,
            eml_path: row.get(8)?,
            has_attachments: row.get(9)?,
            is_from_me: row.get(10)?,
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
