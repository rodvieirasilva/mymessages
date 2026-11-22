use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use serde::Deserialize;

use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant, MediaType};

#[derive(Debug, Deserialize)]
struct UbackTelegramRecord {
    #[serde(default)]
    id: Option<serde_json::Value>,
    #[serde(default)]
    chat_id: Option<String>,
    #[serde(default)]
    chat_name: Option<String>,
    #[serde(default)]
    from: Option<String>,
    #[serde(default)]
    from_id: Option<String>,
    #[serde(default, alias = "text")]
    body: String,
    #[serde(default)]
    date_unixtime: Option<i64>,
    #[serde(default, rename = "dateMs")]
    date_ms: Option<i64>,
    #[serde(default)]
    is_from_me: bool,
}

/// Scan `<account>/telegram/` (SQLite `deltas/` `.db` files, JSON exports,
/// and media files) into `CanonicalMessage` items with `source = "telegram"`.
pub fn read_telegram_dir(tg_root: &Path, account: &str) -> Result<Vec<CanonicalMessage>> {
    if !tg_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut messages = Vec::new();
    let mut seen_db_keys: HashSet<(i64, i64)> = HashSet::new();

    // 1. Read SQLite delta DBs from `deltas/`
    let mut db_files = Vec::new();
    let deltas_dir = tg_root.join("deltas");
    if deltas_dir.is_dir() {
        collect_files_with_ext(&deltas_dir, "db", &mut db_files);
    }
    db_files.sort();

    for db_path in db_files {
        if let Ok(db_msgs) = read_telegram_sqlite(&db_path, account, &mut seen_db_keys) {
            messages.extend(db_msgs);
        }
    }

    // 2. Scan JSON exports & media files (excluding .db / -wal / -shm)
    let mut files = Vec::new();
    collect_files(tg_root, &mut files);
    files.sort();

    for path in files {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

        if matches!(ext.as_str(), "db" | "db-wal" | "db-shm" | "sqlite" | "sqlite3") {
            continue;
        }
        let name_str = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
        if name_str.ends_with(".db-wal") || name_str.ends_with(".db-shm") {
            continue;
        }

        if ext == "json" {
            if let Ok(content) = fs::read_to_string(&path) {
                if let Ok(records) = serde_json::from_str::<Vec<UbackTelegramRecord>>(&content) {
                    for (idx, rec) in records.into_iter().enumerate() {
                        let body = rec.body.trim().to_string();
                        if body.is_empty() {
                            continue;
                        }
                        let chat_id = rec.chat_id.unwrap_or_else(|| "telegram".to_string());
                        let ts_secs = rec
                            .date_unixtime
                            .or_else(|| rec.date_ms.map(|ms| if ms > 10_000_000_000 { ms / 1000 } else { ms }))
                            .unwrap_or(0);
                        let timestamp = DateTime::from_timestamp(ts_secs, 0).unwrap_or_else(Utc::now);
                        let msg_id = rec
                            .id
                            .map(|v| format!("tg_{}", v.to_string().trim_matches('"')))
                            .unwrap_or_else(|| format!("tg_{}_{}", ts_secs, idx));

                        messages.push(CanonicalMessage {
                            id: msg_id,
                            source: "telegram".to_string(),
                            account: account.to_string(),
                            conversation_id: chat_id.clone(),
                            conversation_name: rec.chat_name.clone().or_else(|| Some(chat_id.clone())),
                            is_group: false,
                            sender: CanonicalParticipant {
                                id: rec.from_id.unwrap_or_else(|| if rec.is_from_me { "me".into() } else { chat_id.clone() }),
                                name: rec.from.or_else(|| rec.chat_name.clone()),
                                phone_number: None,
                            },
                            recipients: Vec::new(),
                            timestamp,
                            body: Some(body),
                            attachments: Vec::new(),
                            reply_to_id: None,
                            is_from_me: rec.is_from_me,
                            raw_payload: serde_json::json!({ "uback_telegram_json": path.display().to_string() }),
                        });
                    }
                }
            }
            continue;
        }

        // Media file (images, documents, etc.)
        let file_name = name_str.to_string();
        let mime = match ext.as_str() {
            "jpg" | "jpeg" => "image/jpeg",
            "png" => "image/png",
            "webp" => "image/webp",
            "gif" => "image/gif",
            "mp4" => "video/mp4",
            "ogg" | "opus" => "audio/ogg",
            "mp3" => "audio/mpeg",
            "pdf" => "application/pdf",
            _ => "application/octet-stream",
        };
        let modified = fs::metadata(&path)
            .and_then(|m| m.modified())
            .map(DateTime::<Utc>::from)
            .unwrap_or_else(|_| Utc::now());
        let id_hash = format!("tg_media_{}", simple_hash(&path.to_string_lossy()));

        messages.push(CanonicalMessage {
            id: id_hash,
            source: "telegram".to_string(),
            account: account.to_string(),
            conversation_id: "telegram_media".to_string(),
            conversation_name: Some("Telegram Media".to_string()),
            is_group: false,
            sender: CanonicalParticipant {
                id: "telegram".to_string(),
                name: Some("Telegram".to_string()),
                phone_number: None,
            },
            recipients: Vec::new(),
            timestamp: modified,
            body: None,
            attachments: vec![CanonicalAttachment {
                filename: Some(file_name),
                mimetype: mime.to_string(),
                sha256: String::new(),
                media_type: MediaType::from_mimetype(mime),
                duration_seconds: None,
                file_size: fs::metadata(&path).map(|m| m.len()).unwrap_or(0),
                data: Vec::new(),
                external_path: Some(path.clone()),
            }],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({ "uback_telegram_media": path.display().to_string() }),
        });
    }

    Ok(messages)
}

fn read_telegram_sqlite(
    db_path: &Path,
    account: &str,
    seen: &mut HashSet<(i64, i64)>,
) -> Result<Vec<CanonicalMessage>> {
    let conn = Connection::open_with_flags(db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;

    let mut chats: HashMap<i64, (String, bool)> = HashMap::new();
    if let Ok(mut stmt) = conn.prepare("SELECT id, title, type FROM chats") {
        if let Ok(rows) = stmt.query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
            ))
        }) {
            for row in rows.flatten() {
                let (id, title, chat_type) = row;
                let name = title.filter(|t| !t.trim().is_empty()).unwrap_or_else(|| id.to_string());
                let is_group = chat_type
                    .as_deref()
                    .map(|t| t.contains("Group") || t.contains("Supergroup"))
                    .unwrap_or(false);
                chats.insert(id, (name, is_group));
            }
        }
    }

    let mut out = Vec::new();
    let mut stmt = conn.prepare(
        "SELECT id, chat_id, sender_id, date, content_type, text FROM messages ORDER BY date ASC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, i64>(1)?,
            row.get::<_, Option<String>>(2)?,
            row.get::<_, i64>(3)?,
            row.get::<_, Option<String>>(4)?,
            row.get::<_, Option<String>>(5)?,
        ))
    })?;

    for row in rows.flatten() {
        let (msg_id, chat_id, sender_id_opt, date_raw, content_type_opt, text_opt) = row;
        if !seen.insert((chat_id, msg_id)) {
            continue;
        }
        let body = text_opt
            .map(|t| t.trim().to_string())
            .filter(|t| !t.is_empty())
            .or_else(|| content_type_opt.as_ref().map(|ct| format!("[{}]", ct)));
        let (chat_name, is_group) = chats
            .get(&chat_id)
            .cloned()
            .unwrap_or_else(|| (chat_id.to_string(), false));
        let sender_id = sender_id_opt.unwrap_or_else(|| chat_id.to_string());
        let ts_secs = if date_raw > 10_000_000_000 { date_raw / 1000 } else { date_raw };
        let timestamp = DateTime::from_timestamp(ts_secs, 0).unwrap_or_else(Utc::now);

        out.push(CanonicalMessage {
            id: format!("tg_{}_{}", chat_id, msg_id),
            source: "telegram".to_string(),
            account: account.to_string(),
            conversation_id: format!("tg_{}", chat_id),
            conversation_name: Some(chat_name.clone()),
            is_group,
            sender: CanonicalParticipant {
                id: sender_id.clone(),
                name: Some(if is_group { sender_id } else { chat_name }),
                phone_number: None,
            },
            recipients: Vec::new(),
            timestamp,
            body,
            attachments: Vec::new(),
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({
                "uback_telegram_db": db_path.display().to_string(),
                "content_type": content_type_opt,
            }),
        });
    }

    Ok(out)
}

fn simple_hash(s: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

fn collect_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files(&path, out);
        } else if path.is_file() {
            out.push(path);
        }
    }
}

fn collect_files_with_ext(dir: &Path, ext: &str, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_files_with_ext(&path, ext, out);
        } else if path.is_file()
            && path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case(ext)).unwrap_or(false)
        {
            out.push(path);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_telegram_dir_sqlite_deltas_json_and_media() {
        let dir = std::env::temp_dir().join("mymessages_test_uback_tg");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("deltas/2026/11")).unwrap();
        fs::create_dir_all(dir.join("images")).unwrap();

        let db_path = dir.join("deltas/2026/11/telegram_delta_20261122.db");
        let conn = Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE chats (id INTEGER PRIMARY KEY, title TEXT, type TEXT);
             CREATE TABLE messages (id INTEGER, chat_id INTEGER, sender_id TEXT, date INTEGER, content_type TEXT, text TEXT, PRIMARY KEY (chat_id, id));
             INSERT INTO chats VALUES (100, 'Rust Brasil', 'ChatTypeSupergroup');
             INSERT INTO messages VALUES (1, 100, 'user_42', 1700000100, 'MessageText', 'Hello from Telegram delta');",
        )
        .unwrap();
        drop(conn);

        // Duplicate delta in deltas/ should be deduplicated
        fs::copy(&db_path, dir.join("deltas/2026/11/telegram_delta_20261123.db")).unwrap();

        fs::write(
            dir.join("messages.json"),
            r#"[{"id":"2","chat_id":"tg_200","chat_name":"Alice","from":"Alice","body":"Oi JSON","date_unixtime":1700000000,"is_from_me":false}]"#,
        )
        .unwrap();
        fs::write(dir.join("images/photo.jpg"), b"fakejpg").unwrap();

        let msgs = read_telegram_dir(&dir, "acct1").unwrap();
        assert_eq!(msgs.len(), 3);
        assert!(msgs.iter().all(|m| m.source == "telegram"));
        let delta_msg = msgs.iter().find(|m| m.body.as_deref() == Some("Hello from Telegram delta")).unwrap();
        assert!(delta_msg.is_group);
        assert_eq!(delta_msg.conversation_name.as_deref(), Some("Rust Brasil"));

        let _ = fs::remove_dir_all(&dir);
    }
}
