use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use rusqlite::{Connection, OpenFlags};

#[derive(Debug, Clone)]
pub struct WaContact {
    pub display_name: Option<String>,
    pub phone_number: Option<String>,
}

pub fn read_wa_contacts(wa_db_path: &Path) -> Result<HashMap<String, WaContact>> {
    let conn = Connection::open_with_flags(wa_db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("Failed to open wa.db at {:?}", wa_db_path))?;

    let mut contacts = HashMap::new();
    let has_table: bool = conn
        .query_row(
            "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name='wa_contacts'",
            [],
            |r| r.get(0),
        )
        .unwrap_or(false);
    if !has_table {
        return Ok(contacts);
    }

    let mut stmt = conn.prepare(
        "SELECT jid, display_name, number FROM wa_contacts WHERE jid IS NOT NULL",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, Option<String>>(1)?,
            row.get::<_, Option<String>>(2)?,
        ))
    })?;

    for row in rows.flatten() {
        let (jid, display_name, number) = row;
        contacts.insert(
            jid,
            WaContact {
                display_name: display_name.filter(|s| !s.trim().is_empty()),
                phone_number: number.filter(|s| !s.trim().is_empty()),
            },
        );
    }
    Ok(contacts)
}
