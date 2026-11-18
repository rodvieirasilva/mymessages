pub mod attachments;
pub mod chats;
pub mod health;
pub mod mcp;
pub mod openapi;
pub mod search;
pub mod state;
pub mod webhook;

use std::path::PathBuf;
use axum::http::HeaderValue;
use axum::routing::{any, get, post};
use axum::Router;
use tower_http::cors::{AllowOrigin, Any, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

pub use state::AppState;

pub fn create_router(state: AppState, static_dir: Option<PathBuf>) -> Router {
    let origins = &state.config.server.cors_allowed_origins;
    let cors = if origins.contains(&"*".to_string()) {
        CorsLayer::new()
            .allow_origin(Any)
            .allow_methods(Any)
            .allow_headers(Any)
    } else {
        let parsed_origins: Vec<HeaderValue> = origins
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
        CorsLayer::new()
            .allow_origin(AllowOrigin::list(parsed_origins))
            .allow_methods(Any)
            .allow_headers(Any)
    };

    let api_routes = Router::new()
        .route("/health", get(health::health_handler))
        .route("/search", any(search::search_unified_handler))
        .route("/chats", get(chats::list_chats_handler))
        .route("/chats/{id}/messages", get(chats::get_chat_messages_handler))
        .route("/attachments/{id}", get(attachments::get_attachment_handler))
        .route("/mcp", post(mcp::mcp_http_handler))
        .route("/webhook/evolution", post(webhook::evolution_webhook_handler))
        .route("/webhook/waha", post(webhook::waha_webhook_handler));

    let mut app = Router::new()
        .route("/health", get(health::health_handler))
        .nest("/v1", api_routes.clone())
        .nest("/api/v1", api_routes)
        .with_state(state)
        .merge(openapi::swagger_ui())
        .layer(cors);

    if let Some(dir) = static_dir {
        if dir.exists() {
            let index_file = dir.join("index.html");
            let serve_service = ServeDir::new(&dir).fallback(ServeFile::new(index_file));
            app = app.fallback_service(serve_service);
        }
    }

    app
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use tower::ServiceExt;

    use crate::config::AppConfig;
    use crate::index::Database;

    #[tokio::test]
    async fn test_api_routes_smoke() {
        let db = Database::in_memory().unwrap();
        let mut cfg = AppConfig::default();
        cfg.server.cors_allowed_origins = vec!["http://localhost:5173".into()];
        let state = AppState::new(db, std::env::temp_dir(), cfg);
        let router = create_router(state, None);

        // 1. Versioned routes (/v1) and legacy (/api/v1)
        for path in ["/v1/chats", "/api/v1/chats", "/v1/search?query=hello", "/v1/chats/chat_test/messages"] {
            let req = Request::builder().uri(path).body(Body::empty()).unwrap();
            let resp = router.clone().oneshot(req).await.unwrap();
            assert_eq!(resp.status(), StatusCode::OK, "Failed on route {}", path);
        }

        // 2. Attachment route (404 expected when ID not found)
        let att_req = Request::builder().uri("/v1/attachments/unknown_att").body(Body::empty()).unwrap();
        let att_resp = router.clone().oneshot(att_req).await.unwrap();
        assert_eq!(att_resp.status(), StatusCode::NOT_FOUND);

        // 3. Webhook endpoints
        let waha_req = Request::builder()
            .method("POST")
            .uri("/v1/webhook/waha")
            .header("Content-Type", "application/json")
            .body(Body::from(r#"{"event":"other"}"#))
            .unwrap();
        let waha_resp = router.clone().oneshot(waha_req).await.unwrap();
        assert_eq!(waha_resp.status(), StatusCode::OK);

        // 4. OpenAPI spec endpoint
        let openapi_req = Request::builder()
            .uri("/api-docs/openapi.json")
            .body(Body::empty())
            .unwrap();
        let openapi_resp = router.clone().oneshot(openapi_req).await.unwrap();
        assert_eq!(openapi_resp.status(), StatusCode::OK);

        // 5. Swagger UI endpoint & static UI assets
        let swagger_req = Request::builder().uri("/swagger-ui/").body(Body::empty()).unwrap();
        let swagger_resp = router.clone().oneshot(swagger_req).await.unwrap();
        assert_eq!(swagger_resp.status(), StatusCode::OK);

        let css_req = Request::builder().uri("/swagger-ui/swagger-ui.css").body(Body::empty()).unwrap();
        let css_resp = router.oneshot(css_req).await.unwrap();
        assert_eq!(css_resp.status(), StatusCode::OK);
    }
}
