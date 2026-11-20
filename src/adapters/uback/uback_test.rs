use std::collections::HashMap;
use std::fs;
use std::path::Path;
use rusqlite::Connection;

use super::crypt15::tests::encrypt_crypt15_for_test;
use super::*;

fn create_synthetic_msgstore(path: &Path, chat_jid: &str, body: &str, media_rel: Option<&str>) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "
        CREATE TABLE jid (_id INTEGER PRIMARY KEY, raw_string TEXT);
        CREATE TABLE chat (_id INTEGER PRIMARY KEY, jid_row_id INTEGER, subject TEXT);
        CREATE TABLE message (
            _id INTEGER PRIMARY KEY, chat_row_id INTEGER, from_me INTEGER,
            key_id TEXT, sender_jid_row_id INTEGER, timestamp INTEGER, text_data TEXT
        );
        CREATE TABLE message_media (
            _id INTEGER PRIMARY KEY, message_row_id INTEGER, file_path TEXT,
            mime_type TEXT, media_name TEXT, media_caption TEXT,
            file_size INTEGER, media_duration INTEGER, file_hash TEXT
        );
        ",
    )
    .unwrap();

    conn.execute("INSERT INTO jid (_id, raw_string) VALUES (1, ?1)", [chat_jid]).unwrap();
    conn.execute("INSERT INTO chat (_id, jid_row_id, subject) VALUES (1, 1, NULL)", []).unwrap();
    conn.execute(
        "INSERT INTO message (_id, chat_row_id, from_me, key_id, sender_jid_row_id, timestamp, text_data)
         VALUES (1, 1, 0, 'WA_MSG_1', 1, 1740000000000, ?1)",
        [body],
    )
    .unwrap();

    if let Some(rel) = media_rel {
        let fname = rel.rsplit('/').next().unwrap_or("IMG.jpg");
        conn.execute(
            "INSERT INTO message_media (_id, message_row_id, file_path, mime_type, media_name, file_size, file_hash)
             VALUES (1, 1, ?1, 'image/jpeg', ?2, 12, 'hash_img_1')",
            [rel, fname],
        )
        .unwrap();
    }
}

fn create_synthetic_wa_db(path: &Path, jid: &str, display_name: &str) {
    let conn = Connection::open(path).unwrap();
    conn.execute_batch(
        "CREATE TABLE wa_contacts (_id INTEGER PRIMARY KEY, jid TEXT, display_name TEXT, number TEXT);",
    )
    .unwrap();
    conn.execute(
        "INSERT INTO wa_contacts (_id, jid, display_name, number) VALUES (1, ?1, ?2, '+5511999990000')",
        [jid, display_name],
    )
    .unwrap();
}

#[test]
fn test_uback_multi_account_crypt15_zero_duplication() {
    let root = std::env::temp_dir().join(format!("mymessages_uback_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let key_a = "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef";
    let key_b = "fedcba9876543210fedcba9876543210fedcba9876543210fedcba9876543210";

    let acct_a = root.join("phone_alpha/whatsapp");
    fs::create_dir_all(acct_a.join("databases/2025/02")).unwrap();
    fs::create_dir_all(acct_a.join("backups/2025/02")).unwrap();
    fs::create_dir_all(acct_a.join("received/images/2025/02")).unwrap();

    let media_file = acct_a.join("received/images/2025/02/IMG-20250219-WA0001.jpg");
    fs::write(&media_file, b"fakejpegdata").unwrap();

    let raw_msg_a = root.join("tmp_msg_a.db");
    create_synthetic_msgstore(
        &raw_msg_a,
        "5511999990000@s.whatsapp.net",
        "Check this receipt photo",
        Some("Media/WhatsApp Images/IMG-20250219-WA0001.jpg"),
    );
    let enc_msg_a = encrypt_crypt15_for_test(&fs::read(&raw_msg_a).unwrap(), key_a, &[3u8; 16], true, false);
    fs::write(acct_a.join("databases/2025/02/msgstore.db.crypt15"), enc_msg_a).unwrap();

    let raw_wa_a = root.join("tmp_wa_a.db");
    create_synthetic_wa_db(&raw_wa_a, "5511999990000@s.whatsapp.net", "Alice Contact");
    let enc_wa_a = encrypt_crypt15_for_test(&fs::read(&raw_wa_a).unwrap(), key_a, &[4u8; 16], false, true);
    fs::write(acct_a.join("backups/2025/02/wa.db.crypt15"), enc_wa_a).unwrap();

    let acct_b = root.join("phone_beta/whatsapp");
    fs::create_dir_all(acct_b.join("databases/2025/03")).unwrap();
    let raw_msg_b = root.join("tmp_msg_b.db");
    create_synthetic_msgstore(&raw_msg_b, "5511888880000@s.whatsapp.net", "Second account message", None);
    let enc_msg_b = encrypt_crypt15_for_test(&fs::read(&raw_msg_b).unwrap(), key_b, &[5u8; 12], false, false);
    fs::write(acct_b.join("databases/2025/03/msgstore.db.crypt15"), enc_msg_b).unwrap();

    let mut cred_map = HashMap::new();
    cred_map.insert("WHATSAPP_64DIGI_phone_alpha".into(), key_a.into());
    cred_map.insert("WHATSAPP_64DIGI_phone_beta".into(), key_b.into());
    let creds = CredentialsStore::from_map(cred_map);

    let adapter = UbackAdapter::new(root.clone(), creds, Vec::new(), false);
    let summary = adapter.scan_all().unwrap();

    assert_eq!(summary.accounts_scanned, vec!["phone_alpha", "phone_beta"]);
    assert_eq!(summary.messages.len(), 2);

    let m_a = summary.messages.iter().find(|m| m.account == "phone_alpha").unwrap();
    assert_eq!(m_a.source, "whatsapp");
    assert_eq!(m_a.conversation_name.as_deref(), Some("Alice Contact"));
    assert_eq!(m_a.attachments.len(), 1);
    assert!(m_a.attachments[0].is_external());
    assert!(m_a.attachments[0].data.is_empty());
    assert_eq!(m_a.attachments[0].external_path.as_deref(), Some(media_file.as_path()));

    let m_b = summary.messages.iter().find(|m| m.account == "phone_beta").unwrap();
    assert_eq!(m_b.body.as_deref(), Some("Second account message"));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_uback_media_only_mode_without_db() {
    let root = std::env::temp_dir().join(format!("mymessages_uback_mediaonly_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let sent_dir = root.join("device_only/whatsapp/sent/documents/2025/05");
    fs::create_dir_all(&sent_dir).unwrap();
    let pdf = sent_dir.join("invoice.pdf");
    fs::write(&pdf, b"%PDF-1.4").unwrap();

    let adapter = UbackAdapter::new(root.clone(), CredentialsStore::default(), Vec::new(), true);
    let summary = adapter.scan_all().unwrap();
    assert_eq!(summary.messages.len(), 1);
    assert_eq!(summary.messages[0].account, "device_only");
    assert!(summary.messages[0].is_from_me);
    assert_eq!(summary.messages[0].attachments[0].external_path.as_deref(), Some(pdf.as_path()));

    let _ = fs::remove_dir_all(&root);
}

#[test]
fn test_uback_incremental_sqlite_deltas_with_optional_tables() {
    let root = std::env::temp_dir().join(format!("mymessages_uback_deltas_{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);

    let acct = root.join("phone_delta/whatsapp");
    fs::create_dir_all(acct.join("deltas/2026/01")).unwrap();
    fs::create_dir_all(acct.join("deltas/2026/02")).unwrap();
    fs::create_dir_all(acct.join("received/images/2026/02")).unwrap();

    // Delta 1: text-only delta WITHOUT message_media table (matches UBack Option 1A)
    let delta1 = acct.join("deltas/2026/01/msgstore_delta_20260115_1_10.db");
    let conn1 = Connection::open(&delta1).unwrap();
    conn1.execute_batch(
        "CREATE TABLE delta_meta (scope TEXT, source_file TEXT, exported_at_ms INT, from_row_id INT, to_row_id INT, message_count INT, message_table TEXT);
         CREATE TABLE jid (_id INTEGER PRIMARY KEY, raw_string TEXT);
         CREATE TABLE chat (_id INTEGER PRIMARY KEY, jid_row_id INTEGER, subject TEXT);
         CREATE TABLE message (_id INTEGER PRIMARY KEY, chat_row_id INTEGER, from_me INTEGER, key_id TEXT, sender_jid_row_id INTEGER, timestamp INTEGER, text_data TEXT);
         INSERT INTO jid VALUES (1, '5511000999@s.whatsapp.net');
         INSERT INTO chat VALUES (1, 1, 'Delta Chat');
         INSERT INTO message VALUES (1, 1, 0, 'DELTA_MSG_1', 1, 1768435200000, 'Archived message later deleted on phone');",
    ).unwrap();
    drop(conn1);

    // Delta 2: newer delta WITH media linked to received/images/2026/02/
    let img_path = acct.join("received/images/2026/02/IMG-20260201-WA0002.jpg");
    fs::write(&img_path, b"delta_img_bytes").unwrap();
    let delta2 = acct.join("deltas/2026/02/msgstore_delta_20260201_11_20.db");
    create_synthetic_msgstore(
        &delta2,
        "5511000999@s.whatsapp.net",
        "Newer delta photo",
        Some("Media/WhatsApp Images/IMG-20260201-WA0002.jpg"),
    );

    let adapter = UbackAdapter::new(root.clone(), CredentialsStore::default(), vec![], false);
    let summary = adapter.scan_all().unwrap();
    assert_eq!(summary.accounts_scanned, vec!["phone_delta"]);
    assert_eq!(summary.messages.len(), 2);
    assert!(summary.messages.iter().any(|m| m.body.as_deref() == Some("Archived message later deleted on phone")));

    let m2 = summary.messages.iter().find(|m| m.body.as_deref() == Some("Newer delta photo")).unwrap();
    assert_eq!(m2.attachments.len(), 1);
    assert_eq!(m2.attachments[0].external_path.as_deref(), Some(img_path.as_path()));
    assert!(m2.attachments[0].data.is_empty());

    let _ = fs::remove_dir_all(&root);
}
