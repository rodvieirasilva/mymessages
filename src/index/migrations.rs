use anyhow::Result;
use rusqlite::Connection;

pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "001_perf_indexes",
        sql: "
            CREATE INDEX IF NOT EXISTS idx_messages_conv_ts ON messages(conversation_id, timestamp DESC);
            CREATE INDEX IF NOT EXISTS idx_messages_source ON messages(source_id);
            CREATE INDEX IF NOT EXISTS idx_attachments_msg ON attachments(message_id);
        ",
    },
];

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS schema_migrations (
            version INTEGER PRIMARY KEY,
            name TEXT NOT NULL,
            applied_at INTEGER NOT NULL DEFAULT (unixepoch())
        );",
    )?;

    for m in MIGRATIONS {
        let already: bool = conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = ?1)",
            [m.version],
            |r| r.get(0),
        )?;
        if !already {
            conn.execute_batch(m.sql)?;
            conn.execute(
                "INSERT INTO schema_migrations (version, name) VALUES (?1, ?2)",
                rusqlite::params![m.version, m.name],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::Database;

    #[test]
    fn test_run_migrations_idempotent() {
        let db = Database::in_memory().unwrap();
        run_migrations(db.conn()).unwrap();
        let count: i64 = db
            .conn()
            .query_row("SELECT COUNT(*) FROM schema_migrations", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, MIGRATIONS.len() as i64);
    }
}
