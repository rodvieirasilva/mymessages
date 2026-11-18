use super::*;
use std::fs;
use serde_json::json;
use crate::config::{AppConfig, StorageEntry};
use crate::index::Database;

#[tokio::test]
async fn test_webhook_handlers_and_disabled() {
    let temp_dir = std::env::temp_dir().join(format!("webhook_test_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db = Database::open(&temp_dir.join("test.sqlite3")).unwrap();
    let mut cfg = AppConfig::default();
    cfg.adapters.evolution.enabled = false;
    cfg.adapters.waha.enabled = false;
    cfg.storages = vec![
        StorageEntry::new_maildir("maildir", temp_dir.join("mail")),
        StorageEntry::new_sqlite("sqlite", temp_dir.join("test.sqlite3")),
    ];

    let state = AppState::new(db, temp_dir.clone(), cfg);

    // Test evolution disabled
    let err_evo = evolution_webhook_handler(State(state.clone()), Json(json!({}))).await;
    assert_eq!(err_evo.err().unwrap().0, StatusCode::FORBIDDEN);

    // Test waha disabled
    let err_waha = waha_webhook_handler(State(state.clone()), Json(json!({}))).await;
    assert_eq!(err_waha.err().unwrap().0, StatusCode::FORBIDDEN);

    // Enable WAHA and test ingestion
    let mut enabled_cfg = AppConfig::default();
    enabled_cfg.adapters.waha.enabled = true;
    enabled_cfg.storages = vec![
        StorageEntry::new_maildir("maildir", temp_dir.join("mail2")),
        StorageEntry::new_sqlite("sqlite", temp_dir.join("test2.sqlite3")),
    ];
    let fresh_db = Database::open(&temp_dir.join("test2.sqlite3")).unwrap();
    let enabled_state = AppState::new(fresh_db, temp_dir.clone(), enabled_cfg);

    let waha_payload = json!({
        "event": "message",
        "session": "default",
        "payload": {
            "id": "waha_test_msg_01",
            "timestamp": 1700000000,
            "from": "5511999998888@c.us",
            "to": "5511988887777@c.us",
            "fromMe": false,
            "body": "Test message from WAHA webhook",
            "hasMedia": false
        }
    });

    let resp = waha_webhook_handler(State(enabled_state.clone()), Json(waha_payload.clone())).await.unwrap();
    let body = resp.into_response();
    assert_eq!(body.status(), StatusCode::OK);

    // Second time should be duplicate_skipped
    let dup_resp = waha_webhook_handler(State(enabled_state), Json(waha_payload)).await.unwrap();
    assert_eq!(dup_resp.into_response().status(), StatusCode::OK);

    let _ = fs::remove_dir_all(&temp_dir);
}
