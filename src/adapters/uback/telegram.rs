use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};

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

/// Scan `<account>/telegram/` (media files + any JSON exports) into `CanonicalMessage`
/// items with `source = "telegram"`.
pub fn read_telegram_dir(tg_root: &Path, account: &str) -> Result<Vec<CanonicalMessage>> {
    if !tg_root.is_dir() {
        return Ok(Vec::new());
    }
    let mut messages = Vec::new();
    let mut files = Vec::new();
    collect_files(tg_root, &mut files);
    files.sort();

    for path in files {
        let ext = path
            .extension()
            .and_then(|s| s.to_str())
            .map(|s| s.to_ascii_lowercase())
            .unwrap_or_default();

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
        let file_name = path
            .file_name()
            .and_then(|s| s.to_str())
            .unwrap_or("attachment")
            .to_string();
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
        let id_hash = format!(
            "tg_media_{}",
            blake3_or_simple_hash(&path.to_string_lossy())
        );

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
                media_type: crate::models::MediaType::from_mimetype(mime),
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

fn blake3_or_simple_hash(s: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_telegram_dir_json_and_media() {
        let dir = std::env::temp_dir().join("mymessages_test_uback_tg");
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("images")).unwrap();
        fs::write(
            dir.join("messages.json"),
            r#"[{"id":"1","chat_id":"tg_100","chat_name":"Alice","from":"Alice","body":"Oi Telegram","date_unixtime":1700000000,"is_from_me":false}]"#,
        )
        .unwrap();
        fs::write(dir.join("images/photo.jpg"), b"fakejpg").unwrap();

        let msgs = read_telegram_dir(&dir, "acct1").unwrap();
        assert_eq!(msgs.len(), 2);
        assert!(msgs.iter().all(|m| m.source == "telegram"));
        assert!(msgs.iter().any(|m| m.body.as_deref() == Some("Oi Telegram")));
        assert!(msgs.iter().any(|m| !m.attachments.is_empty()));

        let _ = fs::remove_dir_all(&dir);
    }
}
