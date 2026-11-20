use anyhow::{Context, Result};
use rusqlite::Connection;

use crate::models::{scoped_id, CanonicalMessage, Contact, Conversation};

pub fn index_message(
    conn: &mut Connection,
    msg: &CanonicalMessage,
    content_hash: &str,
    eml_path: &str,
) -> Result<()> {
    let tx = conn.transaction()?;
    let account = if msg.account.is_empty() { "default" } else { msg.account.as_str() };

    // 1. Ensure source & account exist
    tx.execute(
        "INSERT OR IGNORE INTO sources (id, name) VALUES (?1, ?2)",
        [&msg.source, &msg.source],
    )?;
    tx.execute(
        "INSERT OR IGNORE INTO accounts (id, name, source_id) VALUES (?1, ?2, ?3)",
        [account, account, msg.source.as_str()],
    )?;

    // 2. Ensure conversation exists
    let conv = Conversation::for_account(
        &msg.source,
        account,
        &msg.conversation_id,
        msg.conversation_name.clone(),
        msg.is_group,
    );
    tx.execute(
        "INSERT INTO conversations (id, source_id, account_id, remote_id, name, is_group, created_at, updated_at)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, CURRENT_TIMESTAMP, CURRENT_TIMESTAMP)
         ON CONFLICT(id) DO UPDATE SET name = coalesce(?5, name), updated_at = CURRENT_TIMESTAMP",
        rusqlite::params![conv.id, conv.source_id, conv.account_id, conv.remote_id, conv.name, conv.is_group],
    )?;

    // 3. Ensure sender contact exists
    let contact = Contact::for_account(
        &msg.source,
        account,
        &msg.sender.id,
        msg.sender.name.clone(),
        msg.sender.phone_number.clone(),
    );
    tx.execute(
        "INSERT INTO contacts (id, source_id, account_id, remote_id, display_name, phone_number)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)
         ON CONFLICT(id) DO UPDATE SET display_name = coalesce(?5, display_name), phone_number = coalesce(?6, phone_number)",
        rusqlite::params![contact.id, contact.source_id, contact.account_id, contact.remote_id, contact.display_name, contact.phone_number],
    )?;

    // 4. Insert message
    let full_msg_id = scoped_id(&msg.source, account, &msg.id);
    let raw_json_str = serde_json::to_string(&msg.raw_payload).unwrap_or_default();
    tx.execute(
        "INSERT INTO messages (id, source_id, account_id, conversation_id, sender_id, timestamp, body, has_attachments, is_from_me, reply_to_id, eml_path, content_hash, raw_json)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
        rusqlite::params![
            full_msg_id,
            msg.source,
            account,
            conv.id,
            contact.id,
            msg.timestamp.timestamp(),
            msg.body,
            !msg.attachments.is_empty(),
            msg.is_from_me,
            msg.reply_to_id,
            eml_path,
            content_hash,
            raw_json_str,
        ],
    )?;

    // 5. Insert attachments & gather attachment filenames for FTS
    let mut att_names = Vec::new();
    for (idx, att) in msg.attachments.iter().enumerate() {
        let att_id = format!("{}_{}_{}", full_msg_id, att.sha256, idx);
        let fname = att.filename.as_deref();
        if let Some(name) = fname {
            att_names.push(name);
        }
        let file_size = if att.file_size > 0 { att.file_size as i64 } else { att.data.len() as i64 };
        let ext_path = att.external_path.as_ref().map(|p| p.to_string_lossy().to_string());
        tx.execute(
            "INSERT OR REPLACE INTO attachments (id, message_id, filename, mimetype, file_size, sha256, media_type, duration_seconds, external_path)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                att_id,
                full_msg_id,
                fname,
                att.mimetype,
                file_size,
                att.sha256,
                att.media_type.as_str(),
                att.duration_seconds,
                ext_path,
            ],
        )?;
    }

    // 6. Extract and insert metadata
    let metadata_entries = crate::index::metadata::extract_metadata(&msg.raw_payload);
    for (k, v) in &metadata_entries {
        tx.execute(
            "INSERT INTO message_metadata (message_id, key, value) VALUES (?1, ?2, ?3)",
            rusqlite::params![full_msg_id, k, v],
        )?;
    }
    let metadata_text = crate::index::metadata::format_metadata_text(&metadata_entries);

    // 7. Insert into FTS5 virtual table
    let att_names_str = att_names.join(" ");
    tx.execute(
        "INSERT INTO messages_fts (message_id, conversation_name, sender_name, body, attachment_filenames, metadata_text)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![
            full_msg_id,
            msg.conversation_name.as_deref().unwrap_or(""),
            msg.sender.name.as_deref().unwrap_or(""),
            msg.body.as_deref().unwrap_or(""),
            att_names_str,
            metadata_text,
        ],
    )?;

    // 8. Record indexed hash
    tx.execute(
        "INSERT OR IGNORE INTO indexed_hashes (content_hash) VALUES (?1)",
        [content_hash],
    )?;

    tx.commit().context("Failed to commit indexed message transaction")?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::db::Database;
    use crate::models::{CanonicalAttachment, CanonicalParticipant};
    use chrono::Utc;

    #[test]
    fn test_index_message_success() {
        let mut db = Database::in_memory().unwrap();
        let att = CanonicalAttachment::new(Some("doc.pdf".into()), "application/pdf".into(), b"pdfdata".to_vec());
        let msg = CanonicalMessage {
            id: "m_1".into(),
            source: "whatsapp".into(),
            account: "default".into(),
            conversation_id: "c_1".into(),
            conversation_name: Some("Project Group".into()),
            is_group: true,
            sender: CanonicalParticipant {
                id: "u_1".into(),
                name: Some("Bob".into()),
                phone_number: Some("+123".into()),
            },
            recipients: vec![],
            timestamp: Utc::now(),
            body: Some("Contract review attached".into()),
            attachments: vec![att],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({}),
        };

        index_message(db.conn_mut(), &msg, "hash999", "mail/wh/c_1/cur/m.eml").unwrap();

        let count: i64 = db.conn().query_row("SELECT count(*) FROM messages", [], |r| r.get(0)).unwrap();
        assert_eq!(count, 1);

        let fts_count: i64 = db.conn().query_row("SELECT count(*) FROM messages_fts WHERE messages_fts MATCH 'Contract'", [], |r| r.get(0)).unwrap();
        assert_eq!(fts_count, 1);
    }
}
