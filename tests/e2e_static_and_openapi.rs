use std::fs;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use tower::ServiceExt;

use mymessages::api::create_router;
use mymessages::api::state::AppState;
use mymessages::config::AppConfig;
use mymessages::index::Database;

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
    assert!(body_str.contains("favicon.svg"));
    assert!(body_str.contains("favicon.ico"));

    // 2. Request favicons
    let req_fav_svg = Request::builder().uri("/favicon.svg").body(Body::empty()).unwrap();
    let resp_fav_svg = router.clone().oneshot(req_fav_svg).await.unwrap();
    assert_eq!(resp_fav_svg.status(), StatusCode::OK);

    let req_fav_ico = Request::builder().uri("/favicon.ico").body(Body::empty()).unwrap();
    let resp_fav_ico = router.clone().oneshot(req_fav_ico).await.unwrap();
    assert_eq!(resp_fav_ico.status(), StatusCode::OK);

    // 3. Request health endpoint
    let req_health = Request::builder().uri("/health").body(Body::empty()).unwrap();
    let resp_health = router.clone().oneshot(req_health).await.unwrap();
    assert_eq!(resp_health.status(), StatusCode::OK);

    // 4. Request SPA route (fallback to index.html)
    let req_spa = Request::builder().uri("/chat/12345").body(Body::empty()).unwrap();
    let resp_spa = router.clone().oneshot(req_spa).await.unwrap();
    assert_eq!(resp_spa.status(), StatusCode::OK);

    // 5. Request OpenAPI JSON schema
    let req_openapi = Request::builder().uri("/api-docs/openapi.json").body(Body::empty()).unwrap();
    let resp_openapi = router.clone().oneshot(req_openapi).await.unwrap();
    assert_eq!(resp_openapi.status(), StatusCode::OK);
    let openapi_bytes = resp_openapi.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&openapi_bytes).contains("mymessages API"));

    // 6. Request Swagger UI documentation & asset bundle
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

    // 7. Request versioned API routes (/v1 and /api/v1 aliases)
    let req_v1 = Request::builder().uri("/v1/chats?source=whatsapp&query=Charlie").body(Body::empty()).unwrap();
    let resp_v1 = router.clone().oneshot(req_v1).await.unwrap();
    assert_eq!(resp_v1.status(), StatusCode::OK);

    let req_api_v1 = Request::builder().uri("/api/v1/chats").body(Body::empty()).unwrap();
    let resp_api_v1 = router.oneshot(req_api_v1).await.unwrap();
    assert_eq!(resp_api_v1.status(), StatusCode::OK);

    let _ = fs::remove_dir_all(&temp_dir);
}
