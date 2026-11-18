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

#[path = "webhook_test.rs"]
#[cfg(test)]
mod tests;
