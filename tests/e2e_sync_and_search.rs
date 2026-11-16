use axum::body::Body;
use axum::http::{Request, StatusCode};
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use http_body_util::BodyExt;
use mymessages::api::{create_router, AppState};
use mymessages::config::AppConfig;
use mymessages::index::Database;
use serde_json::Value;
use std::fs;
use tower::ServiceExt;

#[tokio::test]
async fn test_e2e_webhook_sync_dedup_search_and_attachment() {
    let temp_dir = std::env::temp_dir().join(format!("mymessages_e2e_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db_path = temp_dir.join("messages.sqlite3");
    let db = Database::open(&db_path).unwrap();
    let state = AppState::new(db, temp_dir.clone(), AppConfig::default());
    let router = create_router(state, None);

    let image_bytes = b"my test jpeg binary payload 123456";
    let b64_image = BASE64.encode(image_bytes);

    let webhook_payload = serde_json::json!({
        "event": "messages.upsert",
        "data": {
            "key": {
                "remoteJid": "5511988887777@s.whatsapp.net",
                "fromMe": false,
                "id": "E2E_MSG_001"
            },
            "pushName": "Charlie",
            "message": {
                "imageMessage": {
                    "caption": "Project blueprint and proposal",
                    "mimetype": "image/jpeg",
                    "fileName": "blueprint.jpg",
                    "base64": b64_image
                }
            },
            "messageTimestamp": 1710000000
        }
    });

    // 1. Send Webhook
    let req = Request::builder()
        .method("POST")
        .uri("/api/v1/webhook/evolution")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&webhook_payload).unwrap()))
        .unwrap();

    let resp = router.clone().oneshot(req).await.unwrap();
    assert_eq!(resp.status(), StatusCode::OK);

    let body_bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body_bytes).unwrap();
    assert_eq!(json["status"], "indexed");
    assert_eq!(json["message_id"], "E2E_MSG_001");

    // 2. Duplicate Check: Send same webhook again
    let req_dup = Request::builder()
        .method("POST")
        .uri("/api/v1/webhook/evolution")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&webhook_payload).unwrap()))
        .unwrap();

    let resp_dup = router.clone().oneshot(req_dup).await.unwrap();
    assert_eq!(resp_dup.status(), StatusCode::OK);
    let dup_bytes = resp_dup.into_body().collect().await.unwrap().to_bytes();
    let dup_json: Value = serde_json::from_slice(&dup_bytes).unwrap();
    assert_eq!(dup_json["status"], "duplicate_skipped");

    // 3. Search for "blueprint"
    let req_search = Request::builder()
        .uri("/api/v1/search?query=blueprint")
        .body(Body::empty())
        .unwrap();

    let resp_search = router.clone().oneshot(req_search).await.unwrap();
    assert_eq!(resp_search.status(), StatusCode::OK);
    let search_bytes = resp_search.into_body().collect().await.unwrap().to_bytes();
    let search_results: Value = serde_json::from_slice(&search_bytes).unwrap();

    assert!(search_results.as_array().unwrap().len() >= 1);
    let first = &search_results[0];
    assert_eq!(first["sender_name"], "Charlie");
    assert_eq!(first["body"], "Project blueprint and proposal");
    assert_eq!(first["attachments"][0]["filename"], "blueprint.jpg");

    let att_id = first["attachments"][0]["id"].as_str().unwrap();

    // 4. Download attachment
    let req_att = Request::builder()
        .uri(format!("/api/v1/attachments/{}", att_id))
        .body(Body::empty())
        .unwrap();

    let resp_att = router.clone().oneshot(req_att).await.unwrap();
    assert_eq!(resp_att.status(), StatusCode::OK);
    assert_eq!(resp_att.headers().get("content-type").unwrap(), "image/jpeg");
    let downloaded_bytes = resp_att.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&downloaded_bytes[..], image_bytes);

    // 5. List chats
    let req_chats = Request::builder()
        .uri("/api/v1/chats")
        .body(Body::empty())
        .unwrap();
    let resp_chats = router.clone().oneshot(req_chats).await.unwrap();
    assert_eq!(resp_chats.status(), StatusCode::OK);
    let chats_bytes = resp_chats.into_body().collect().await.unwrap().to_bytes();
    let chats: Value = serde_json::from_slice(&chats_bytes).unwrap();
    assert_eq!(chats[0]["name"], "Charlie");
    assert_eq!(chats[0]["message_count"], 1);

    let _ = fs::remove_dir_all(&temp_dir);
}

#[tokio::test]
async fn test_e2e_static_files_served_by_rust() {
    let temp_dir = std::env::temp_dir().join(format!("mymessages_static_{}", std::process::id()));
    let _ = fs::remove_dir_all(&temp_dir);
    fs::create_dir_all(&temp_dir).unwrap();

    let db_path = temp_dir.join("messages.sqlite3");
    let db = Database::open(&db_path).unwrap();
    let state = AppState::new(db, temp_dir.clone(), AppConfig::default());

    let static_dir = std::path::PathBuf::from("web/dist");
    let router = create_router(state, Some(static_dir));

    // 1. Request root index.html
    let req_root = Request::builder().uri("/").body(Body::empty()).unwrap();
    let resp_root = router.clone().oneshot(req_root).await.unwrap();
    assert_eq!(resp_root.status(), StatusCode::OK);

    let body_bytes = resp_root.into_body().collect().await.unwrap().to_bytes();
    let body_str = String::from_utf8_lossy(&body_bytes);
    assert!(body_str.contains("<!doctype html>") || body_str.contains("<html"));

    // 2. Request SPA route (fallback to index.html)
    let req_spa = Request::builder().uri("/chat/12345").body(Body::empty()).unwrap();
    let resp_spa = router.clone().oneshot(req_spa).await.unwrap();
    assert_eq!(resp_spa.status(), StatusCode::OK);

    let _ = fs::remove_dir_all(&temp_dir);
}
