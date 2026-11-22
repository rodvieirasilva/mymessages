use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};
use anyhow::{bail, Context, Result};
use chrono::{DateTime, Utc};
use sha2::{Digest, Sha256};

use crate::adapters::uback::crypt15;
use crate::adapters::uback::media_index::{guess_mimetype, UbackMediaIndex};
use crate::config::CredentialsStore;
use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};

pub struct DbHandle {
    path: PathBuf,
    cleanup: bool,
}

impl DbHandle {
    pub fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for DbHandle {
    fn drop(&mut self) {
        if self.cleanup {
            let _ = fs::remove_file(&self.path);
            let base = self.path.to_string_lossy();
            let _ = fs::remove_file(format!("{}-shm", base));
            let _ = fs::remove_file(format!("{}-wal", base));
        }
    }
}

pub fn open_or_decrypt_db(db_path: &Path, account: &str, creds: &CredentialsStore) -> Result<DbHandle> {
    let name = db_path
        .file_name()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();

    if name.ends_with(".db") || name.ends_with(".sqlite") || name.ends_with(".sqlite3") {
        return Ok(DbHandle { path: db_path.to_path_buf(), cleanup: false });
    }

    if name.ends_with(".crypt15") {
        let hex_key = creds.whatsapp_key_for(account).with_context(|| {
            format!("Missing WHATSAPP_64DIGI_{} key in environment or credentials file for {:?}", account, db_path)
        })?;
        let encrypted = fs::read(db_path)
            .with_context(|| format!("Failed to read encrypted DB {:?}", db_path))?;
        let plain = crypt15::decrypt_crypt15(&encrypted, &hex_key)?;
        let tmp_name = format!(
            "mymessages_dec_{}_{}_{}.sqlite",
            account,
            std::process::id(),
            Utc::now().timestamp_nanos_opt().unwrap_or(0)
        );
        let tmp_path = std::env::temp_dir().join(tmp_name);
        fs::write(&tmp_path, plain)?;
        return Ok(DbHandle { path: tmp_path, cleanup: true });
    }

    bail!("Unsupported WhatsApp database format {:?} (supported: .db and .crypt15)", db_path)
}

/// Find ALL UBack incremental SQLite delta files (`deltas/**/msgstore_delta_*.db`)
/// sorted chronologically by filename/path so earlier deltas (including later-deleted
/// messages) are processed in order.
pub fn find_all_delta_dbs(deltas_root: &Path) -> Vec<PathBuf> {
    let mut deltas = Vec::new();
    if deltas_root.is_dir() {
        collect_delta_files(deltas_root, &mut deltas);
    }
    deltas.sort();
    deltas
}

fn collect_delta_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_delta_files(&path, out);
        } else if path.is_file() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            if name.starts_with("msgstore_delta_") && name.ends_with(".db") {
                out.push(path);
            }
        }
    }
}

pub fn find_latest_db(root: &Path, prefix: &str) -> Option<PathBuf> {
    if !root.is_dir() {
        return None;
    }
    let mut candidates: Vec<(i64, PathBuf)> = Vec::new();
    collect_db_files(root, prefix, &mut candidates);
    candidates.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| b.1.cmp(&a.1)));
    candidates.into_iter().next().map(|(_, p)| p)
}

fn collect_db_files(dir: &Path, prefix: &str, out: &mut Vec<(i64, PathBuf)>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_db_files(&path, prefix, out);
        } else if path.is_file() {
            let name = entry.file_name().to_string_lossy().to_lowercase();
            let matches_prefix = name.starts_with(prefix) && !name.contains("increment") && !name.starts_with("msgstore_delta_");
            let supported_ext = name.ends_with(".db.crypt15") || name.ends_with(".db");
            if matches_prefix && supported_ext {
                let mtime = entry
                    .metadata()
                    .ok()
                    .and_then(|m| m.modified().ok())
                    .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                    .map(|d| d.as_secs() as i64)
                    .unwrap_or(0);
                out.push((mtime, path));
            }
        }
    }
}

pub fn media_entries_to_messages(
    account: &str,
    source: &str,
    media_index: &UbackMediaIndex,
) -> Vec<CanonicalMessage> {
    let empty = HashSet::new();
    media_index
        .unlinked(&empty)
        .into_iter()
        .map(|entry| {
            let mime = guess_mimetype(&entry.filename).to_string();
            let mut h = Sha256::new();
            h.update(entry.path.to_string_lossy().as_bytes());
            h.update(entry.file_size.to_le_bytes());
            let sha = hex::encode(h.finalize());
            let att = CanonicalAttachment::from_external(
                Some(entry.filename.clone()),
                mime,
                entry.path.clone(),
                entry.file_size,
                sha.clone(),
            );
            let ts = DateTime::from_timestamp(entry.modified_ts, 0).unwrap_or_else(Utc::now);
            let conv_id = if entry.is_sent { "uback_sent_media" } else { "uback_received_media" };
            let conv_name = if entry.is_sent { "Sent Media (UBack)" } else { "Received Media (UBack)" };
            CanonicalMessage {
                id: format!("uback_media_{}", &sha[..16]),
                source: source.to_string(),
                account: account.to_string(),
                conversation_id: conv_id.to_string(),
                conversation_name: Some(conv_name.to_string()),
                is_group: false,
                sender: CanonicalParticipant {
                    id: if entry.is_sent { "me".into() } else { "uback_media".into() },
                    name: Some(if entry.is_sent { account.to_string() } else { "Media Archive".into() }),
                    phone_number: None,
                },
                recipients: Vec::new(),
                timestamp: ts,
                body: Some(entry.filename.clone()),
                attachments: vec![att],
                reply_to_id: None,
                is_from_me: entry.is_sent,
                raw_payload: serde_json::json!({ "adapter": "uback_media", "account": account }),
            }
        })
        .collect()
}
