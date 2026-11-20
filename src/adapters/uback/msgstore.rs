use std::collections::HashMap;
use std::path::Path;
use anyhow::{Context, Result};
use chrono::{DateTime, Utc};
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};

use crate::adapters::uback::media_index::{guess_mimetype, UbackMediaIndex};
use crate::adapters::uback::wa_contacts::{lookup_contact, WaContact};
use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};

fn table_exists(conn: &Connection, name: &str) -> bool {
    conn.query_row(
        "SELECT count(*) > 0 FROM sqlite_master WHERE type='table' AND name=?1",
        [name],
        |r| r.get(0),
    )
    .unwrap_or(false)
}

pub fn read_msgstore(
    msgstore_path: &Path,
    account: &str,
    source: &str,
    contacts: &HashMap<String, WaContact>,
    media_index: &UbackMediaIndex,
) -> Result<Vec<CanonicalMessage>> {
    let conn = Connection::open_with_flags(msgstore_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .with_context(|| format!("Failed to open msgstore.db at {:?}", msgstore_path))?;

    let sql = if table_exists(&conn, "message") {
        let has_chat = table_exists(&conn, "chat");
        let has_jid = table_exists(&conn, "jid");
        let has_media = table_exists(&conn, "message_media");
        let chat_jid_expr = if has_chat && has_jid { "COALESCE(cj.raw_string, 'unknown@s.whatsapp.net')" } else { "'unknown@s.whatsapp.net'" };
        let subject_expr = if has_chat { "c.subject" } else { "NULL" };
        let sender_expr = if has_jid { "COALESCE(sj.raw_string, '')" } else { "''" };
        let media_cols = if has_media {
            "mm.file_path, mm.mime_type, mm.media_name, mm.media_caption, mm.file_size, mm.media_duration, mm.file_hash"
        } else {
            "NULL, NULL, NULL, NULL, NULL, NULL, NULL"
        };
        let mut joins = String::new();
        if has_chat { joins.push_str(" LEFT JOIN chat c ON c._id = m.chat_row_id"); }
        if has_chat && has_jid { joins.push_str(" LEFT JOIN jid cj ON cj._id = c.jid_row_id"); }
        if has_jid { joins.push_str(" LEFT JOIN jid sj ON sj._id = m.sender_jid_row_id"); }
        if has_media { joins.push_str(" LEFT JOIN message_media mm ON mm.message_row_id = m._id"); }
        format!(
            "SELECT COALESCE(m.key_id, CAST(m._id AS TEXT)), {chat_jid_expr}, {subject_expr},
                    COALESCE(m.from_me, 0), {sender_expr}, m.timestamp, m.text_data, {media_cols}
             FROM message m{joins}
             WHERE m.timestamp IS NOT NULL AND m.timestamp > 0
             ORDER BY m.timestamp ASC"
        )
    } else if table_exists(&conn, "messages") {
        let has_chat_list = table_exists(&conn, "chat_list");
        let subject_expr = if has_chat_list { "cl.subject" } else { "NULL" };
        let join = if has_chat_list { " LEFT JOIN chat_list cl ON cl.key_remote_jid = m.key_remote_jid" } else { "" };
        format!(
            "SELECT COALESCE(m.key_id, CAST(m._id AS TEXT)), COALESCE(m.key_remote_jid, 'unknown@s.whatsapp.net'),
                    {subject_expr}, COALESCE(m.key_from_me, 0), COALESCE(m.remote_resource, ''),
                    m.timestamp, m.data, m.media_name, m.media_mime_type, m.media_name,
                    m.media_caption, m.media_size, m.media_duration, m.media_hash
             FROM messages m{join}
             WHERE m.timestamp IS NOT NULL AND m.timestamp > 0
             ORDER BY m.timestamp ASC"
        )
    } else {
        return Ok(Vec::new());
    };

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok(RawWaRow {
            msg_id: row.get(0)?,
            chat_jid: row.get(1)?,
            chat_subject: row.get(2)?,
            is_from_me: row.get::<_, i64>(3)? != 0,
            sender_jid: row.get(4)?,
            timestamp_ms: row.get(5)?,
            text_data: row.get(6)?,
            file_path: row.get(7)?,
            mime_type: row.get(8)?,
            media_name: row.get(9)?,
            media_caption: row.get(10)?,
            file_size: row.get(11)?,
            media_duration: row.get(12)?,
            file_hash: row.get(13)?,
        })
    })?;

    Ok(rows
        .flatten()
        .filter_map(|r| r.into_canonical(account, source, contacts, media_index))
        .collect())
}

struct RawWaRow {
    msg_id: String,
    chat_jid: String,
    chat_subject: Option<String>,
    is_from_me: bool,
    sender_jid: String,
    timestamp_ms: i64,
    text_data: Option<String>,
    file_path: Option<String>,
    mime_type: Option<String>,
    media_name: Option<String>,
    media_caption: Option<String>,
    file_size: Option<i64>,
    media_duration: Option<i64>,
    file_hash: Option<String>,
}

impl RawWaRow {
    fn into_canonical(
        self,
        account: &str,
        source: &str,
        contacts: &HashMap<String, WaContact>,
        media_index: &UbackMediaIndex,
    ) -> Option<CanonicalMessage> {
        let body = self.text_data.filter(|s| !s.is_empty()).or_else(|| self.media_caption.filter(|s| !s.is_empty()));
        let mut attachments = Vec::new();
        if let Some(entry) = media_index.resolve(self.file_path.as_deref(), self.media_name.as_deref(), self.is_from_me) {
            let mime = self.mime_type.filter(|m| !m.is_empty()).unwrap_or_else(|| guess_mimetype(&entry.filename).to_string());
            let sha = self.file_hash.filter(|h| !h.is_empty()).unwrap_or_else(|| path_fallback_hash(&entry.path, entry.file_size));
            let size = if entry.file_size > 0 { entry.file_size } else { self.file_size.unwrap_or(0).max(0) as u64 };
            let mut att = CanonicalAttachment::from_external(Some(entry.filename.clone()), mime, entry.path.clone(), size, sha);
            if let Some(dur) = self.media_duration.filter(|d| *d > 0) {
                att = att.with_duration(dur as u32);
            }
            attachments.push(att);
        }
        if body.is_none() && attachments.is_empty() { return None; }

        let is_group = self.chat_jid.ends_with("@g.us");
        let ts_secs = if self.timestamp_ms > 10_000_000_000 { self.timestamp_ms / 1000 } else { self.timestamp_ms };
        let timestamp = DateTime::from_timestamp(ts_secs, 0).unwrap_or_else(Utc::now);
        let sender_jid = if self.is_from_me { "me".to_string() } else if !self.sender_jid.is_empty() { self.sender_jid } else { self.chat_jid.clone() };
        let sender_contact = lookup_contact(contacts, &sender_jid);
        let chat_contact = lookup_contact(contacts, &self.chat_jid);

        let sender_name = if self.is_from_me {
            Some(account.to_string())
        } else {
            sender_contact.and_then(|c| c.display_name.clone()).or_else(|| Some(jid_short(&sender_jid)))
        };
        let sender_phone = sender_contact.and_then(|c| c.phone_number.clone()).or_else(|| jid_phone(&sender_jid));
        let conversation_name = if is_group {
            self.chat_subject.filter(|s| !s.is_empty()).or_else(|| chat_contact.and_then(|c| c.display_name.clone())).or_else(|| Some(jid_short(&self.chat_jid)))
        } else {
            chat_contact.and_then(|c| c.display_name.clone()).or_else(|| self.chat_subject.filter(|s| !s.is_empty())).or_else(|| Some(jid_short(&self.chat_jid)))
        };

        Some(CanonicalMessage {
            id: self.msg_id,
            source: source.to_string(),
            account: account.to_string(),
            conversation_id: self.chat_jid,
            conversation_name,
            is_group,
            sender: CanonicalParticipant { id: sender_jid, name: sender_name, phone_number: sender_phone },
            recipients: Vec::new(),
            timestamp,
            body,
            attachments,
            reply_to_id: None,
            is_from_me: self.is_from_me,
            raw_payload: serde_json::json!({ "adapter": "uback", "account": account, "source": source }),
        })
    }
}

fn jid_short(jid: &str) -> String { jid.split('@').next().unwrap_or(jid).to_string() }
fn jid_phone(jid: &str) -> Option<String> {
    if jid.ends_with("@s.whatsapp.net") || jid.ends_with("@c.us") { Some(jid_short(jid)) } else { None }
}
fn path_fallback_hash(path: &Path, size: u64) -> String {
    let mut h = Sha256::new();
    h.update(path.to_string_lossy().as_bytes());
    h.update(size.to_le_bytes());
    hex::encode(h.finalize())
}
