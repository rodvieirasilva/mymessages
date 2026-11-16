use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mymessages::api::{create_router, AppState};
use mymessages::config::AppConfig;
use mymessages::index::{reindex_from_folder, reindex_from_maildir, Database};
use mymessages::storage::MaildirStorage;
use serde_json::Value;
use std::fs;
use std::path::Path;
use tower::ServiceExt;

#[tokio::test]
async fn test_e2e_folder_adapter_ingest_and_reindex() {
    let temp_dir = std::env::temp_dir().join(format!("mymessages_folder_e2e_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db_path = temp_dir.join("messages.sqlite3");
    let mut db = Database::open(&db_path).unwrap();
    let storage = MaildirStorage::new(&temp_dir);

    // 1. Ingest from test_messages folder
    let report = reindex_from_folder(
        db.conn_mut(),
        &storage,
        Path::new("./test_messages"),
        true,
        true,
    ).unwrap();

    assert!(report.total_found >= 4);
    assert_eq!(report.newly_indexed, report.total_found);

    // 2. Start API router backed by this database & storage
    let state = AppState::new(db, temp_dir.clone(), AppConfig::default());
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

    // 5. Test attachment download
    let att_id = contract_res[0]["attachments"][0]["id"].as_str().unwrap();
    let req_att = Request::builder()
        .uri(format!("/api/v1/attachments/{}", att_id))
        .body(Body::empty())
        .unwrap();
    let resp_att = router.clone().oneshot(req_att).await.unwrap();
    assert_eq!(resp_att.status(), StatusCode::OK);
    assert_eq!(resp_att.headers().get("content-type").unwrap(), "application/pdf");

    // 6. Test complete DB recovery: delete SQLite DB, recreate from Maildir
    drop(router);
    fs::remove_file(&db_path).unwrap();
    assert!(!db_path.exists());

    let mut restored_db = Database::open(&db_path).unwrap();
    let restore_report = reindex_from_maildir(restored_db.conn_mut(), &temp_dir).unwrap();
    assert_eq!(restore_report.newly_indexed, report.total_found);

    // Verify search works on restored database
    let restored_count: i64 = restored_db.conn().query_row("SELECT count(*) FROM messages", [], |r| r.get(0)).unwrap();
    assert_eq!(restored_count, report.total_found as i64);

    let _ = fs::remove_dir_all(&temp_dir);
}
