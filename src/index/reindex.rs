use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::{DateTime, Utc};
use rusqlite::Connection;

use crate::adapters::FolderAdapter;
use crate::index::hasher::is_hash_indexed;
use crate::index::writer::index_message;
use crate::models::{CanonicalMessage, CanonicalParticipant};
use crate::storage::StoragePipeline;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReindexSource {
    Maildir,
    Folder(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReindexTarget {
    Db,
    Maildir,
    All,
}

pub struct ReindexReport {
    pub total_found: usize,
    pub newly_indexed: usize,
    pub duplicates_skipped: usize,
}

pub fn reindex_from_maildir(
    conn: &mut Connection,
    mail_dir: &Path,
) -> Result<ReindexReport> {
    let mut report = ReindexReport {
        total_found: 0,
        newly_indexed: 0,
        duplicates_skipped: 0,
    };

    if !mail_dir.exists() {
        return Ok(report);
    }

    let mut eml_files = Vec::new();
    find_eml_files(mail_dir, &mut eml_files)?;
    report.total_found = eml_files.len();

    for eml_path in eml_files {
        let content = fs::read_to_string(&eml_path)?;
        if let Some(msg) = parse_eml_to_canonical(&content) {
            let hash = extract_header(&content, "X-MyMessages-SHA256").unwrap_or_else(|| msg.compute_content_hash());
            if is_hash_indexed(conn, &hash)? {
                report.duplicates_skipped += 1;
                continue;
            }

            let rel_path = eml_path.strip_prefix(mail_dir).unwrap_or(&eml_path);
            index_message(conn, &msg, &hash, &rel_path.to_string_lossy())?;
            report.newly_indexed += 1;
        }
    }

    Ok(report)
}

pub fn reindex_from_folder(
    pipeline: &StoragePipeline,
    folder: &Path,
) -> Result<ReindexReport> {
    let adapter = FolderAdapter::new(folder);
    let messages = adapter.read_all_messages()?;
    let mut report = ReindexReport {
        total_found: messages.len(),
        newly_indexed: 0,
        duplicates_skipped: 0,
    };

    for msg in messages {
        let hash = msg.compute_content_hash();
        let res = pipeline.write_all(&msg, &hash)?;
        if res.db_indexed || res.eml_path.is_some() {
            report.newly_indexed += 1;
        } else {
            report.duplicates_skipped += 1;
        }
    }

    Ok(report)
}

fn find_eml_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.is_dir() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            find_eml_files(&path, files)?;
        } else if path.extension().and_then(|s| s.to_str()) == Some("eml") {
            files.push(path);
        }
    }
    Ok(())
}

fn extract_header(eml: &str, header: &str) -> Option<String> {
    let target = format!("{}: ", header);
    for line in eml.lines() {
        if line.starts_with(&target) {
            return Some(line[target.len()..].trim().to_string());
        }
    }
    None
}

pub fn parse_eml_to_canonical(eml: &str) -> Option<CanonicalMessage> {
    let source = extract_header(eml, "X-MyMessages-Source").unwrap_or_else(|| "unknown".into());
    let conv_id = extract_header(eml, "X-MyMessages-Chat-ID").unwrap_or_else(|| "chat".into());
    let msg_id = extract_header(eml, "X-MyMessages-Message-ID")
        .or_else(|| extract_header(eml, "Message-ID"))
        .unwrap_or_else(|| "msg".into());

    let subject = extract_header(eml, "Subject");
    let is_from_me = extract_header(eml, "X-MyMessages-Direction").as_deref() == Some("outgoing");

    let ts_str = extract_header(eml, "X-MyMessages-Timestamp");
    let timestamp = ts_str
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
        .unwrap_or_else(Utc::now);

    let sender_raw = extract_header(eml, "From").unwrap_or_default();
    let sender_name = sender_raw.find('"').and_then(|start| {
        sender_raw[start + 1..].find('"').map(|end| sender_raw[start + 1..start + 1 + end].to_string())
    });

    let body = extract_eml_body(eml);

    Some(CanonicalMessage {
        id: msg_id,
        source,
        conversation_id: conv_id,
        conversation_name: subject,
        is_group: false,
        sender: CanonicalParticipant {
            id: "sender".into(),
            name: sender_name,
            phone_number: None,
        },
        recipients: Vec::new(),
        timestamp,
        body,
        attachments: Vec::new(),
        reply_to_id: None,
        is_from_me,
        raw_payload: serde_json::Value::Null,
    })
}

fn extract_eml_body(eml: &str) -> Option<String> {
    let sep = if eml.contains("\r\n\r\n") { "\r\n\r\n" } else { "\n\n" };
    let parts: Vec<&str> = eml.splitn(2, sep).collect();
    if parts.len() < 2 {
        return None;
    }

    let body_section = parts[1];
    if eml.contains("multipart/mixed") {
        for block in body_section.split("--==") {
            if block.contains("text/plain") {
                if let Some(pos) = block.find(sep) {
                    let text = block[pos + sep.len()..].trim();
                    if !text.is_empty() {
                        return Some(text.to_string());
                    }
                }
            }
        }
    }

    Some(body_section.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Arc, Mutex};
    use crate::config::AppConfig;
    use crate::index::Database;
    use crate::storage::MaildirStorage;

    #[test]
    fn test_reindex_from_folder_and_maildir() {
        let temp_dir = std::env::temp_dir().join(format!("test_reindex_pipe_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        fs::create_dir_all(&temp_dir).unwrap();

        let db = Arc::new(Mutex::new(Database::in_memory().unwrap()));
        let storage = Arc::new(MaildirStorage::new(&temp_dir));
        let mut cfg = AppConfig::default();
        cfg.storages = vec![
            crate::config::StorageEntry::new_maildir("maildir", &temp_dir),
            crate::config::StorageEntry::new_sqlite("sqlite", ":memory:"),
        ];
        let pipeline = StoragePipeline::from_config(&cfg, db.clone(), storage);

        // 1. Ingest from test_messages folder via pipeline
        let report = reindex_from_folder(
            &pipeline,
            Path::new("./tests/test_messages"),
        ).unwrap();

        assert!(report.total_found >= 4);
        assert_eq!(report.newly_indexed, report.total_found);

        // 2. Clear database to simulate complete DB loss
        let mut fresh_db = Database::in_memory().unwrap();
        let restore_report = reindex_from_maildir(fresh_db.conn_mut(), &temp_dir).unwrap();
        assert!(restore_report.newly_indexed >= 4);

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
