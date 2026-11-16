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
    let mail_dir = temp_dir.join("mail");
    let db = Database::open(&db_path).unwrap();

    let mut cfg = AppConfig::default();
    cfg.storages = vec![
        mymessages::config::StorageEntry::new_maildir("maildir", &mail_dir),
        mymessages::config::StorageEntry::new_sqlite("sqlite", &db_path),
    ];

    let state = AppState::new(db, temp_dir.clone(), cfg);
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
    let status = resp_att.status();
    let downloaded_bytes = resp_att.into_body().collect().await.unwrap().to_bytes();
    if status != StatusCode::OK {
        panic!("Attachment error {}: {}", status, String::from_utf8_lossy(&downloaded_bytes));
    }
    assert_eq!(status, StatusCode::OK);
    assert_eq!(&downloaded_bytes[..], image_bytes);

    // 5. List chats
    let req_chats = Request::builder()
        .uri("/v1/chats")
        .body(Body::empty())
        .unwrap();
    let resp_chats = router.clone().oneshot(req_chats).await.unwrap();
    assert_eq!(resp_chats.status(), StatusCode::OK);
    let chats_bytes = resp_chats.into_body().collect().await.unwrap().to_bytes();
    let chats: Value = serde_json::from_slice(&chats_bytes).unwrap();
    assert_eq!(chats[0]["name"], "Charlie");
    assert_eq!(chats[0]["message_count"], 1);

    // 6. Get chat messages
    let chat_id = chats[0]["id"].as_str().unwrap();
    let req_chat_msgs = Request::builder()
        .uri(format!("/v1/chats/{}/messages", chat_id))
        .body(Body::empty())
        .unwrap();
    let resp_chat_msgs = router.clone().oneshot(req_chat_msgs).await.unwrap();
    assert_eq!(resp_chat_msgs.status(), StatusCode::OK);
    let msgs_bytes = resp_chat_msgs.into_body().collect().await.unwrap().to_bytes();
    let msgs: Value = serde_json::from_slice(&msgs_bytes).unwrap();
    assert_eq!(msgs.as_array().unwrap().len(), 1);

    // 7. WAHA Webhook Ingestion & Search
    let waha_payload = serde_json::json!({
        "event": "message",
        "payload": {
            "id": "WAHA_E2E_001",
            "timestamp": 1710000050,
            "from": "5511999998888@c.us",
            "fromMe": false,
            "body": "Hello from WAHA test message",
            "_data": {
                "notifyName": "Diana"
            }
        }
    });
    let req_waha = Request::builder()
        .method("POST")
        .uri("/v1/webhook/waha")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&waha_payload).unwrap()))
        .unwrap();
    let resp_waha = router.clone().oneshot(req_waha).await.unwrap();
    assert_eq!(resp_waha.status(), StatusCode::OK);

    let req_search_waha = Request::builder()
        .uri("/v1/search?query=WAHA")
        .body(Body::empty())
        .unwrap();
    let resp_search_waha = router.clone().oneshot(req_search_waha).await.unwrap();
    assert_eq!(resp_search_waha.status(), StatusCode::OK);
    let waha_search_bytes = resp_search_waha.into_body().collect().await.unwrap().to_bytes();
    let waha_res: Value = serde_json::from_slice(&waha_search_bytes).unwrap();
    assert_eq!(waha_res.as_array().unwrap().len(), 1);
    assert_eq!(waha_res[0]["sender_name"], "Diana");

    // 8. HTTP QUERY Method with Elasticsearch JSON Body
    let query_dsl_body = serde_json::json!({
        "query": {
            "bool": {
                "must": [
                    { "match": { "body": "blueprint" } }
                ],
                "filter": [
                    { "term": { "media_type": "image" } }
                ]
            }
        }
    });

    let req_query_dsl = Request::builder()
        .method("QUERY")
        .uri("/v1/search")
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_vec(&query_dsl_body).unwrap()))
        .unwrap();
    let resp_query_dsl = router.clone().oneshot(req_query_dsl).await.unwrap();
    assert_eq!(resp_query_dsl.status(), StatusCode::OK);
    let q_bytes = resp_query_dsl.into_body().collect().await.unwrap().to_bytes();
    let q_res: Value = serde_json::from_slice(&q_bytes).unwrap();
    assert_eq!(q_res.as_array().unwrap().len(), 1);
    assert_eq!(q_res[0]["sender_name"], "Charlie");

    // 9. Lucene Query String Filter
    let req_lucene = Request::builder()
        .uri("/v1/search?query=blueprint%20from:Charlie%20type:image")
        .body(Body::empty())
        .unwrap();
    let resp_lucene = router.clone().oneshot(req_lucene).await.unwrap();
    assert_eq!(resp_lucene.status(), StatusCode::OK);
    let lucene_bytes = resp_lucene.into_body().collect().await.unwrap().to_bytes();
    let lucene_res: Value = serde_json::from_slice(&lucene_bytes).unwrap();
    assert_eq!(lucene_res.as_array().unwrap().len(), 1);

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

    // 3. Request OpenAPI JSON schema
    let req_openapi = Request::builder().uri("/api-docs/openapi.json").body(Body::empty()).unwrap();
    let resp_openapi = router.clone().oneshot(req_openapi).await.unwrap();
    assert_eq!(resp_openapi.status(), StatusCode::OK);
    let openapi_bytes = resp_openapi.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&openapi_bytes).contains("mymessages API"));

    // 4. Request Swagger UI documentation & asset bundle
    let req_swagger = Request::builder().uri("/swagger-ui/").body(Body::empty()).unwrap();
    let resp_swagger = router.clone().oneshot(req_swagger).await.unwrap();
    assert_eq!(resp_swagger.status(), StatusCode::OK);
    let swagger_bytes = resp_swagger.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&swagger_bytes).contains("swagger-ui"));

    let req_bundle = Request::builder().uri("/swagger-ui/swagger-ui-bundle.js").body(Body::empty()).unwrap();
    let resp_bundle = router.clone().oneshot(req_bundle).await.unwrap();
    assert_eq!(resp_bundle.status(), StatusCode::OK);

    let req_css = Request::builder().uri("/swagger-ui/swagger-ui.css").body(Body::empty()).unwrap();
    let resp_css = router.clone().oneshot(req_css).await.unwrap();
    assert_eq!(resp_css.status(), StatusCode::OK);

    let req_preset = Request::builder().uri("/swagger-ui/swagger-ui-standalone-preset.js").body(Body::empty()).unwrap();
    let resp_preset = router.clone().oneshot(req_preset).await.unwrap();
    assert_eq!(resp_preset.status(), StatusCode::OK);

    // 5. Request versioned API routes (/v1 and /api/v1 aliases)
    let req_v1 = Request::builder().uri("/v1/chats").body(Body::empty()).unwrap();
    let resp_v1 = router.clone().oneshot(req_v1).await.unwrap();
    assert_eq!(resp_v1.status(), StatusCode::OK);

    let req_api_v1 = Request::builder().uri("/api/v1/chats").body(Body::empty()).unwrap();
    let resp_api_v1 = router.oneshot(req_api_v1).await.unwrap();
    assert_eq!(resp_api_v1.status(), StatusCode::OK);

    let _ = fs::remove_dir_all(&temp_dir);
}
