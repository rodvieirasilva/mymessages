use std::path::Path;
use anyhow::{Context, Result};
use rusqlite::Connection;

pub struct Database {
    conn: Connection,
}

impl Database {
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let conn = Connection::open(path)?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn in_memory() -> Result<Self> {
        let conn = Connection::open_in_memory()?;
        let db = Self { conn };
        db.init_schema()?;
        Ok(db)
    }

    pub fn conn(&self) -> &Connection {
        &self.conn
    }

    pub fn conn_mut(&mut self) -> &mut Connection {
        &mut self.conn
    }

    fn init_schema(&self) -> Result<()> {
        self.conn.execute_batch(
            "
            PRAGMA journal_mode = WAL;
            PRAGMA foreign_keys = ON;

            CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL REFERENCES sources(id),
                remote_id TEXT NOT NULL,
                name TEXT,
                is_group BOOLEAN NOT NULL DEFAULT 0,
                created_at TIMESTAMP,
                updated_at TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS contacts (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL REFERENCES sources(id),
                remote_id TEXT NOT NULL,
                display_name TEXT,
                phone_number TEXT,
                avatar_url TEXT
            );

            CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                source_id TEXT NOT NULL REFERENCES sources(id),
                conversation_id TEXT NOT NULL REFERENCES conversations(id),
                sender_id TEXT NOT NULL REFERENCES contacts(id),
                timestamp INTEGER NOT NULL,
                body TEXT,
                has_attachments BOOLEAN NOT NULL DEFAULT 0,
                is_from_me BOOLEAN NOT NULL DEFAULT 0,
                reply_to_id TEXT,
                eml_path TEXT NOT NULL,
                content_hash TEXT NOT NULL UNIQUE,
                raw_json TEXT,
                created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS attachments (
                id TEXT PRIMARY KEY,
                message_id TEXT NOT NULL REFERENCES messages(id),
                filename TEXT,
                mimetype TEXT NOT NULL,
                file_size INTEGER NOT NULL,
                sha256 TEXT NOT NULL,
                media_type TEXT,
                duration_seconds INTEGER
            );

            CREATE TABLE IF NOT EXISTS indexed_hashes (
                content_hash TEXT PRIMARY KEY,
                indexed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
            );

            CREATE TABLE IF NOT EXISTS message_metadata (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                message_id TEXT NOT NULL REFERENCES messages(id) ON DELETE CASCADE,
                key TEXT NOT NULL,
                value TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_meta_key_val ON message_metadata(key, value);
            CREATE INDEX IF NOT EXISTS idx_meta_msg ON message_metadata(message_id);

            CREATE VIRTUAL TABLE IF NOT EXISTS messages_fts USING fts5(
                message_id UNINDEXED,
                conversation_name,
                sender_name,
                body,
                attachment_filenames,
                metadata_text,
                tokenize='unicode61 remove_diacritics 2'
            );
            ",
        ).context("Failed to initialize database schema")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_database_init() {
        let db = Database::in_memory().unwrap();
        let mut stmt = db.conn().prepare("SELECT name FROM sqlite_master WHERE type='table'").unwrap();
        let tables: Vec<String> = stmt.query_map([], |row| row.get(0)).unwrap().map(|r| r.unwrap()).collect();

        assert!(tables.contains(&"sources".to_string()));
        assert!(tables.contains(&"messages".to_string()));
        assert!(tables.contains(&"messages_fts".to_string()));
    }
}
