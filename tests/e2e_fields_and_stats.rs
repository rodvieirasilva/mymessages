use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use mymessages::api::{create_router, AppState};
use mymessages::config::AppConfig;
use mymessages::index::Database;
use serde_json::Value;
use tower::ServiceExt;

#[tokio::test]
async fn test_e2e_fields_and_stats_endpoints() {
    let dir = std::env::temp_dir().join("mymessages_e2e_fields_stats");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let db = Database::open(dir.join("messages.sqlite3")).unwrap();
    let state = AppState::new(db, dir.clone(), AppConfig::default());
    let app = create_router(state, None);

    let fields_resp = app
        .clone()
        .oneshot(Request::builder().uri("/v1/fields").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(fields_resp.status(), StatusCode::OK);
    let body = fields_resp.into_body().collect().await.unwrap().to_bytes();
    let json: Value = serde_json::from_slice(&body).unwrap();
    let fields = json["fields"].as_array().unwrap();
    assert_eq!(fields.len(), 11);
    assert!(fields.iter().any(|f| f["name"] == "source_id" && f["wildcard_supported"] == true));
    assert!(fields.iter().any(|f| f["name"] == "timestamp" && f["facetable"] == true));

    let stats_resp = app
        .oneshot(Request::builder().uri("/v1/stats").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(stats_resp.status(), StatusCode::OK);
    let s_body = stats_resp.into_body().collect().await.unwrap().to_bytes();
    let s_json: Value = serde_json::from_slice(&s_body).unwrap();
    assert_eq!(s_json["total_messages"], 0);
    assert_eq!(s_json["total_chats"], 0);

    let _ = std::fs::remove_dir_all(&dir);
}
