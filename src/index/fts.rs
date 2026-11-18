use anyhow::{Context, Result};
use rusqlite::Connection;
use crate::index::query_dsl::CompiledQuery;

pub use crate::index::search_models::{AttachmentInfo, SearchQuery, SearchResult};

pub fn search_messages(conn: &Connection, q: &SearchQuery) -> Result<Vec<SearchResult>> {
    let compiled = q.to_compiled();
    search_compiled(conn, &compiled)
}

pub fn search_compiled(conn: &Connection, q: &CompiledQuery) -> Result<Vec<SearchResult>> {
    let match_expr = if q.text_terms.is_empty() {
        String::new()
    } else {
        q.text_terms
            .iter()
            .map(|t| {
                let clean = t.replace('"', "\"\"");
                if clean.starts_with('"') && clean.ends_with('"') {
                    clean
                } else {
                    format!("\"{}\"*", clean)
                }
            })
            .collect::<Vec<_>>()
            .join(" ")
    };
    let has_text = !match_expr.is_empty();

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

    let mut param_index = 12;
    for _ in q.metadata_filters.iter().take(20) {
        sql.push_str(&format!(
            " AND EXISTS (SELECT 1 FROM message_metadata md WHERE md.message_id = m.id AND md.key = ?{} AND md.value = ?{})",
            param_index,
            param_index + 1
        ));
        param_index += 2;
    }

    sql.push_str(&format!(" {} LIMIT ?10 OFFSET ?11", order_clause));

    let mut params: Vec<Box<dyn rusqlite::ToSql>> = vec![
        Box::new(match_expr),
        Box::new(q.source.clone()),
        Box::new(q.chat_id.clone()),
        Box::new(q.has_media),
        Box::new(q.is_from_me),
        Box::new(q.media_type.clone()),
        Box::new(sender_pattern),
        Box::new(q.from_timestamp),
        Box::new(q.to_timestamp),
        Box::new(q.limit as i64),
        Box::new(q.offset as i64),
    ];
    for (k, v) in q.metadata_filters.iter().take(20) {
        params.push(Box::new(k.clone()));
        params.push(Box::new(v.clone()));
    }
    let param_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|b| &**b).collect();

    let mut stmt = conn.prepare(&sql).context("Failed to prepare FTS search query")?;
    let rows = stmt.query_map(&param_refs[..], |row| {
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

#[path = "fts_test.rs"]
#[cfg(test)]
mod tests;
