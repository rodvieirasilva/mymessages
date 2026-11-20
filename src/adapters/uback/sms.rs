use std::fs;
use std::path::{Path, PathBuf};
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::Deserialize;

use crate::models::{CanonicalMessage, CanonicalParticipant};

#[derive(Debug, Deserialize)]
struct UbackSmsRecord {
    #[serde(default)]
    address: String,
    #[serde(default)]
    body: String,
    #[serde(rename = "dateMs", default)]
    date_ms: i64,
    #[serde(rename = "type", default)]
    sms_type: i64,
}

/// Scan `<account>/snapshots/sms/**/sms_*.json` (UBack `SnapshotExporter`) into
/// `CanonicalMessage` items with `source = "sms"`.
pub fn read_sms_snapshots(sms_root: &Path, account: &str) -> Result<Vec<CanonicalMessage>> {
    let mut json_files = Vec::new();
    collect_json_files(sms_root, &mut json_files);
    json_files.sort();

    let mut messages = Vec::new();
    for path in json_files {
        let Ok(content) = fs::read_to_string(&path) else { continue };
        let Ok(records) = serde_json::from_str::<Vec<UbackSmsRecord>>(&content) else { continue };

        for (idx, rec) in records.into_iter().enumerate() {
            let body = rec.body.trim().to_string();
            if body.is_empty() {
                continue;
            }
            let addr = if rec.address.trim().is_empty() {
                "unknown".to_string()
            } else {
                rec.address.trim().to_string()
            };
            let is_from_me = matches!(rec.sms_type, 2 | 4 | 5 | 6);
            let ts_secs = if rec.date_ms > 10_000_000_000 {
                rec.date_ms / 1000
            } else {
                rec.date_ms
            };
            let timestamp = DateTime::from_timestamp(ts_secs, 0).unwrap_or_else(Utc::now);
            let msg_id = format!("sms_{}_{}_{}", sanitize_id(&addr), rec.date_ms, idx);

            messages.push(CanonicalMessage {
                id: msg_id,
                source: "sms".to_string(),
                account: account.to_string(),
                conversation_id: addr.clone(),
                conversation_name: Some(addr.clone()),
                is_group: false,
                sender: CanonicalParticipant {
                    id: if is_from_me { "me".into() } else { addr.clone() },
                    name: Some(if is_from_me { account.to_string() } else { addr.clone() }),
                    phone_number: Some(addr),
                },
                recipients: Vec::new(),
                timestamp,
                body: Some(body),
                attachments: Vec::new(),
                reply_to_id: None,
                is_from_me,
                raw_payload: serde_json::json!({
                    "adapter": "uback_sms",
                    "account": account,
                    "type": rec.sms_type,
                }),
            });
        }
    }

    Ok(messages)
}

fn collect_json_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, out);
        } else if path.is_file()
            && path.extension().and_then(|s| s.to_str()).map(|s| s.eq_ignore_ascii_case("json")).unwrap_or(false)
        {
            out.push(path);
        }
    }
}

fn sanitize_id(raw: &str) -> String {
    raw.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
        .collect()
}
