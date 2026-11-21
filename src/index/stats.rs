use std::path::Path;
use anyhow::Result;
use rusqlite::Connection;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct SourceStats {
    pub source: String,
    pub message_count: i64,
    pub chat_count: i64,
}

#[derive(Debug, Clone, Serialize, ToSchema)]
pub struct VaultStats {
    pub db_path: String,
    pub db_size_bytes: u64,
    pub total_messages: i64,
    pub total_chats: i64,
    pub sources: Vec<SourceStats>,
}

pub fn get_vault_stats(conn: &Connection, db_path: &Path) -> Result<VaultStats> {
    let db_size_bytes = std::fs::metadata(db_path).map(|m| m.len()).unwrap_or(0);
    let total_messages: i64 =
        conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?;
    let total_chats: i64 =
        conn.query_row("SELECT COUNT(DISTINCT conversation_id) FROM messages", [], |r| r.get(0))?;

    let mut stmt = conn.prepare(
        "SELECT source_id, COUNT(*) as msg_cnt, COUNT(DISTINCT conversation_id) as chat_cnt
         FROM messages
         GROUP BY source_id
         ORDER BY msg_cnt DESC",
    )?;
    let sources = stmt
        .query_map([], |row| {
            Ok(SourceStats {
                source: row.get(0)?,
                message_count: row.get(1)?,
                chat_count: row.get(2)?,
            })
        })?
        .collect::<std::result::Result<Vec<_>, _>>()?;

    Ok(VaultStats {
        db_path: db_path.display().to_string(),
        db_size_bytes,
        total_messages,
        total_chats,
        sources,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Database;

    #[test]
    fn test_get_vault_stats_empty_db() {
        let db_path = std::env::temp_dir().join("mymessages_test_stats.sqlite3");
        let _ = std::fs::remove_file(&db_path);
        let db = Database::open(&db_path).unwrap();
        let stats = get_vault_stats(db.conn(), &db_path).unwrap();
        assert_eq!(stats.total_messages, 0);
        assert_eq!(stats.total_chats, 0);
        assert!(stats.sources.is_empty());
        assert!(stats.db_size_bytes > 0);
        let _ = std::fs::remove_file(&db_path);
    }
}
