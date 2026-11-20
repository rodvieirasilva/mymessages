use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use rusqlite::Connection;
use serde_json::Value;
use tower::ServiceExt;

use mymessages::adapters::UbackAdapter;
use mymessages::api::{create_router, AppState};
use mymessages::config::{AppConfig, CredentialsStore, StorageEntry, UbackAdapterConfig};
use mymessages::index::{reindex_from_maildir, Database};
use mymessages::storage::{MaildirStorage, StoragePipeline};

fn build_msgstore(path: &Path, chat_jid: &str, text: &str, media_path: Option<&str>, fname: Option<&str>) {
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
    ).unwrap();
    conn.execute("INSERT INTO jid (_id, raw_string) VALUES (1, ?1)", [chat_jid]).unwrap();
    conn.execute("INSERT INTO chat (_id, jid_row_id, subject) VALUES (1, 1, 'Shared Chat')", []).unwrap();
    conn.execute(
        "INSERT INTO message (_id, chat_row_id, from_me, key_id, sender_jid_row_id, timestamp, text_data)
         VALUES (1, 1, 0, 'SHARED_KEY_1', 1, 1740000000000, ?1)",
        [text],
    ).unwrap();
    if let (Some(mp), Some(fnm)) = (media_path, fname) {
        conn.execute(
            "INSERT INTO message_media (_id, message_row_id, file_path, mime_type, media_name, file_size, file_hash)
             VALUES (1, 1, ?1, 'image/jpeg', ?2, 16, 'sha256_zero_dup')",
            [mp, fnm],
        ).unwrap();
    }
}

#[tokio::test]
async fn test_e2e_uback_multi_account_zero_duplication_and_api() {
    let base = std::env::temp_dir().join(format!("mymessages_e2e_uback_{}", std::process::id()));
    let _ = fs::remove_dir_all(&base);
    let backup_root = base.join("uback_backup");
    let data_dir = base.join("vault_data");
    let mail_dir = data_dir.join("mail");
    fs::create_dir_all(&mail_dir).unwrap();

    // Create two UBack accounts with WhatsApp databases & media
    let acct1_wa = backup_root.join("device_one/whatsapp");
    fs::create_dir_all(acct1_wa.join("databases/2025/04")).unwrap();
    fs::create_dir_all(acct1_wa.join("received/images/2025/04")).unwrap();
    let media_bytes = b"ZERO_DUP_JPEG_16";
    let media_file = acct1_wa.join("received/images/2025/04/IMG-20250401-WA0001.jpg");
    fs::write(&media_file, media_bytes).unwrap();
    build_msgstore(
        &acct1_wa.join("databases/2025/04/msgstore.db"),
        "55110001@s.whatsapp.net",
        "Vacation itinerary photo from device one",
        Some("Media/WhatsApp Images/IMG-20250401-WA0001.jpg"),
        Some("IMG-20250401-WA0001.jpg"),
    );

    let acct2_wa = backup_root.join("device_two/whatsapp");
    fs::create_dir_all(acct2_wa.join("databases/2025/04")).unwrap();
    build_msgstore(
        &acct2_wa.join("databases/2025/04/msgstore.db"),
        "55110002@s.whatsapp.net",
        "Vacation itinerary notes from device two",
        None,
        None,
    );

    // Run UBack adapter into StoragePipeline (Maildir + SQLite)
    let adapter = UbackAdapter::new(backup_root.clone(), CredentialsStore::default(), vec![], false);
    let summary = adapter.scan_all().unwrap();
    assert_eq!(summary.accounts_scanned, vec!["device_one", "device_two"]);
    assert_eq!(summary.messages.len(), 2);

    let db_path = data_dir.join("messages.sqlite3");
    let db = Database::open(&db_path).unwrap();
    let db_arc = Arc::new(Mutex::new(db));
    let storage_arc = Arc::new(MaildirStorage::new(&mail_dir));

    let mut cfg = AppConfig::default();
    cfg.storages = vec![
        StorageEntry::new_maildir("maildir", &mail_dir),
        StorageEntry::new_sqlite("sqlite", &db_path),
    ];
    cfg.adapters.uback = Some(UbackAdapterConfig {
        enabled: true,
        backup_root: backup_root.clone(),
        credentials_file: None,
        accounts: vec![],
        index_media_without_db: false,
        poll_interval_seconds: 30,
    });

    let pipeline = StoragePipeline::from_config(&cfg, db_arc, storage_arc);
    for msg in &summary.messages {
        let hash = msg.compute_content_hash();
        let report = pipeline.write_all(msg, &hash).unwrap();
        assert!(report.db_indexed);
        if msg.account == "device_one" {
            let eml_content = fs::read_to_string(data_dir.join(report.eml_path.unwrap())).unwrap();
            assert!(eml_content.contains("X-Attachment-External-Path:"));
            assert!(!eml_content.contains("Content-Transfer-Encoding: base64"));
        }
    }

    // Serve API & test cross-account search, account facet endpoint, and zero-duplication streaming
    let db_serve = Database::open(&db_path).unwrap();
    let state = AppState::new(db_serve, data_dir.clone(), cfg);
    let router = create_router(state, None);

    // 1. GET /v1/accounts returns both accounts
    let resp = router.clone().oneshot(Request::builder().uri("/v1/accounts").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let accts: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(accts.len(), 2);

    // 2. Cross-account search finds BOTH accounts' messages
    let resp = router.clone().oneshot(Request::builder().uri("/v1/search?query=itinerary").body(Body::empty()).unwrap()).await.unwrap();
    let all_res: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(all_res.len(), 2);

    // 3. Account-filtered search (query param & Lucene)
    let resp = router.clone().oneshot(Request::builder().uri("/v1/search?query=itinerary&account=device_one").body(Body::empty()).unwrap()).await.unwrap();
    let one_res: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(one_res.len(), 1);
    assert_eq!(one_res[0]["account"], "device_one");

    let resp = router.clone().oneshot(Request::builder().uri("/v1/search?query=itinerary%20account:device_two").body(Body::empty()).unwrap()).await.unwrap();
    let two_res: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(two_res.len(), 1);
    assert_eq!(two_res[0]["account"], "device_two");

    // 4. Chats filtered by account, has_media (Text Only vs Media), and direction
    let resp = router.clone().oneshot(Request::builder().uri("/v1/chats?account=device_two").body(Body::empty()).unwrap()).await.unwrap();
    let chats_two: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(chats_two.len(), 1);
    assert_eq!(chats_two[0]["account_id"], "device_two");

    // Text-only chats filter (`has_media=false`) returns only device_two's text-only chat
    let resp = router.clone().oneshot(Request::builder().uri("/v1/chats?has_media=false").body(Body::empty()).unwrap()).await.unwrap();
    let text_chats: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(text_chats.len(), 1);
    assert_eq!(text_chats[0]["account_id"], "device_two");

    // Image chats filter (`media_type=image&has_media=true`) returns only device_one's image chat
    let resp = router.clone().oneshot(Request::builder().uri("/v1/chats?has_media=true&media_type=image").body(Body::empty()).unwrap()).await.unwrap();
    let img_chats: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(img_chats.len(), 1);
    assert_eq!(img_chats[0]["account_id"], "device_one");

    // Incoming direction filter (`is_from_me=false`) returns both chats
    let resp = router.clone().oneshot(Request::builder().uri("/v1/chats?is_from_me=false").body(Body::empty()).unwrap()).await.unwrap();
    let in_chats: Vec<Value> = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(in_chats.len(), 2);

    // Health endpoint includes version, build_time, and commit
    let resp = router.clone().oneshot(Request::builder().uri("/health").body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let health: Value = serde_json::from_slice(&resp.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(health["status"], "ok");
    assert!(health["build_time"].as_str().is_some());
    assert!(health["commit"].as_str().is_some());

    // 5. Attachment streams directly from UBack external path without duplication
    let att_id = one_res[0]["attachments"][0]["id"].as_str().unwrap();
    let resp = router.clone().oneshot(Request::builder().uri(format!("/v1/attachments/{}", att_id)).body(Body::empty()).unwrap()).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let streamed = resp.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&streamed[..], media_bytes);

    // 6. Maildir reindex preserves both account and external_path
    let mut fresh_db = Database::in_memory().unwrap();
    let reindex_rep = reindex_from_maildir(fresh_db.conn_mut(), &mail_dir).unwrap();
    assert_eq!(reindex_rep.newly_indexed, 2);
    let ext_path: Option<String> = fresh_db
        .conn()
        .query_row("SELECT external_path FROM attachments LIMIT 1", [], |r| r.get(0))
        .unwrap();
    assert_eq!(ext_path.as_deref(), Some(media_file.to_string_lossy().as_ref()));

    let _ = fs::remove_dir_all(&base);
}
