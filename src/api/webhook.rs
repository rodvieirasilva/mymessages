use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use serde_json::Value;
use utoipa::ToSchema;

use crate::adapters::evolution::EvolutionApiAdapter;
use crate::adapters::waha::WahaAdapter;
use crate::adapters::SourceAdapter;
use crate::api::state::AppState;
use crate::index::is_hash_indexed;
use crate::models::CanonicalMessage;

#[derive(Debug, Serialize, ToSchema)]
pub struct WebhookResult {
    pub status: String,
    pub message_id: Option<String>,
    pub content_hash: Option<String>,
    pub eml_path: Option<String>,
}

fn ingest_message(
    state: &AppState,
    msg: &CanonicalMessage,
) -> Result<WebhookResult, (StatusCode, String)> {
    let content_hash = msg.compute_content_hash();

    if state.config.is_storage_enabled("sqlite") {
        let db = state.db.lock().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
        })?;
        if is_hash_indexed(db.conn(), &content_hash).unwrap_or(false) {
            return Ok(WebhookResult {
                status: "duplicate_skipped".into(),
                message_id: Some(msg.id.clone()),
                content_hash: Some(content_hash),
                eml_path: None,
            });
        }
    }

    let report = state.pipeline.write_all(msg, &content_hash).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Storage pipeline error: {}", e),
        )
    })?;

    Ok(WebhookResult {
        status: "indexed".into(),
        message_id: Some(msg.id.clone()),
        content_hash: Some(content_hash),
        eml_path: report.eml_path,
    })
}

#[utoipa::path(
    post,
    path = "/v1/webhook/evolution",
    tag = "webhook",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "Webhook processed successfully", body = WebhookResult),
        (status = 400, description = "Invalid webhook payload"),
        (status = 403, description = "Evolution adapter is disabled"),
        (status = 500, description = "Database or storage pipeline error")
    )
)]
pub async fn evolution_webhook_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if !state.config.adapters.evolution.enabled {
        return Err((
            StatusCode::FORBIDDEN,
            "Evolution API adapter is disabled".into(),
        ));
    }

    let adapter = EvolutionApiAdapter::new();
    let msg = match adapter.parse_webhook(&payload) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Ok(Json(WebhookResult {
                status: "ignored".into(),
                message_id: None,
                content_hash: None,
                eml_path: None,
            }))
        }
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse webhook: {}", e),
            ))
        }
    };

    let res = ingest_message(&state, &msg)?;
    Ok(Json(res))
}

#[utoipa::path(
    post,
    path = "/v1/webhook/waha",
    tag = "webhook",
    request_body = serde_json::Value,
    responses(
        (status = 200, description = "WAHA webhook processed successfully", body = WebhookResult),
        (status = 400, description = "Invalid webhook payload"),
        (status = 403, description = "WAHA adapter is disabled"),
        (status = 500, description = "Database or storage pipeline error")
    )
)]
pub async fn waha_webhook_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    if !state.config.adapters.waha.enabled {
        return Err((StatusCode::FORBIDDEN, "WAHA adapter is disabled".into()));
    }

    let adapter = WahaAdapter::new();
    let msg = match adapter.parse_webhook(&payload) {
        Ok(Some(m)) => m,
        Ok(None) => {
            return Ok(Json(WebhookResult {
                status: "ignored".into(),
                message_id: None,
                content_hash: None,
                eml_path: None,
            }))
        }
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse WAHA webhook: {}", e),
            ))
        }
    };

    let res = ingest_message(&state, &msg)?;
    Ok(Json(res))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use serde_json::json;
    use crate::config::AppConfig;
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
}
