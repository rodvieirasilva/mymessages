use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mymessages::api::{create_router, AppState};
use mymessages::config::{AppConfig, StorageEntry};
use mymessages::index::{reindex_from_folder, reindex_from_maildir, Database};
use mymessages::storage::{MaildirStorage, StoragePipeline};
use serde_json::Value;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use tower::ServiceExt;

#[tokio::test]
async fn test_e2e_folder_adapter_ingest_and_reindex() {
    let temp_dir = std::env::temp_dir().join(format!("mymessages_folder_e2e_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db_path = temp_dir.join("messages.sqlite3");
    let mail_dir = temp_dir.join("mail");
    let db = Database::open(&db_path).unwrap();
    let db_arc = Arc::new(Mutex::new(db));
    let storage = Arc::new(MaildirStorage::new(&mail_dir));

    let mut cfg = AppConfig::default();
    cfg.storages = vec![
        StorageEntry::new_maildir("maildir", &mail_dir),
        StorageEntry::new_sqlite("sqlite", &db_path),
    ];
    let pipeline = StoragePipeline::from_config(&cfg, db_arc.clone(), storage.clone());

    // 1. Ingest from test_messages folder
    let report = reindex_from_folder(&pipeline, Path::new("./test_messages")).unwrap();

    assert!(report.total_found >= 4);
    assert_eq!(report.newly_indexed, report.total_found);

    // 2. Start API router backed by this database & storage
    let db_conn = Database::open(&db_path).unwrap();
    let state = AppState::new(db_conn, temp_dir.clone(), cfg);
    let router = create_router(state, None);

    // 3. Search for "architecture"
    let req = Request::builder()
        .uri("/api/v1/search?query=architecture")
        .body(Body::empty())
        .unwrap();
    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let search_res: Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(search_res.as_array().unwrap().len(), 1);
    assert_eq!(search_res[0]["sender_name"], "Bob");

    // 4. Search for "contract"
    let req_contract = Request::builder()
        .uri("/api/v1/search?query=contract")
        .body(Body::empty())
        .unwrap();
    let resp_contract = router.clone().oneshot(req_contract).await.unwrap();
    assert_eq!(resp_contract.status(), StatusCode::OK);
    let bytes_contract = resp_contract.into_body().collect().await.unwrap().to_bytes();
    let contract_res: Value = serde_json::from_slice(&bytes_contract).unwrap();
    assert_eq!(contract_res.as_array().unwrap().len(), 1);

    // 5. Test complete DB recovery: delete SQLite DB, recreate from Maildir
    drop(router);
    fs::remove_file(&db_path).unwrap();
    assert!(!db_path.exists());

    let mut restored_db = Database::open(&db_path).unwrap();
    let restore_report = reindex_from_maildir(restored_db.conn_mut(), &mail_dir).unwrap();
    assert_eq!(restore_report.newly_indexed, report.total_found);

    let restored_count: i64 = restored_db.conn().query_row("SELECT count(*) FROM messages", [], |r| r.get(0)).unwrap();
    assert_eq!(restored_count, report.total_found as i64);

    let _ = fs::remove_dir_all(&temp_dir);
}
