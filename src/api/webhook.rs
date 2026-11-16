use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::Value;

use crate::adapters::evolution::EvolutionApiAdapter;
use crate::adapters::SourceAdapter;
use crate::api::state::AppState;
use crate::index::is_hash_indexed;

pub async fn evolution_webhook_handler(
    State(state): State<AppState>,
    Json(payload): Json<Value>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let adapter = EvolutionApiAdapter::new();
    let msg = match adapter.parse_webhook(&payload) {
        Ok(Some(m)) => m,
        Ok(None) => return Ok(Json(serde_json::json!({ "status": "ignored" }))),
        Err(e) => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("Failed to parse webhook: {}", e),
            ))
        }
    };

    let content_hash = msg.compute_content_hash();

    // Check deduplication
    if state.config.is_storage_enabled("sqlite") {
        let db = state.db.lock().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
        })?;
        if is_hash_indexed(db.conn(), &content_hash).unwrap_or(false) {
            return Ok(Json(serde_json::json!({
                "status": "duplicate_skipped",
                "message_id": msg.id,
                "content_hash": content_hash
            })));
        }
    }

    // Dispatch to modular storage pipeline
    let report = state
        .pipeline
        .write_all(&msg, &content_hash)
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Storage pipeline error: {}", e),
            )
        })?;

    Ok(Json(serde_json::json!({
        "status": "indexed",
        "message_id": msg.id,
        "content_hash": content_hash,
        "eml_path": report.eml_path.unwrap_or_default()
    })))
}
