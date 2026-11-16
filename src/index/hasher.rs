use anyhow::{Context, Result};
use rusqlite::Connection;

pub fn is_hash_indexed(conn: &Connection, content_hash: &str) -> Result<bool> {
    let mut stmt = conn
        .prepare("SELECT 1 FROM indexed_hashes WHERE content_hash = ?1")
        .context("Failed to prepare check indexed hash query")?;
    let exists = stmt.exists([content_hash])?;
    Ok(exists)
}

pub fn record_hash(conn: &Connection, content_hash: &str) -> Result<()> {
    conn.execute(
        "INSERT OR IGNORE INTO indexed_hashes (content_hash) VALUES (?1)",
        [content_hash],
    )
    .context("Failed to record indexed hash")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::db::Database;

    #[test]
    fn test_hasher_record_and_check() {
        let db = Database::in_memory().unwrap();
        let hash = "abc123456789def";

        assert!(!is_hash_indexed(db.conn(), hash).unwrap());
        record_hash(db.conn(), hash).unwrap();
        assert!(is_hash_indexed(db.conn(), hash).unwrap());

        // Idempotent record
        record_hash(db.conn(), hash).unwrap();
        assert!(is_hash_indexed(db.conn(), hash).unwrap());
    }
}
