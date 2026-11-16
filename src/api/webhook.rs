use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::Value;

use crate::adapters::evolution::EvolutionApiAdapter;
use crate::adapters::SourceAdapter;
use crate::api::state::AppState;
use crate::index::{index_message, is_hash_indexed};
use crate::storage::build_eml;

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
    if state.config.storage.enable_db {
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

    // Write to Maildir (if enabled)
    let mut rel_path_str = String::new();
    if state.config.storage.enable_maildir {
        let eml_content = build_eml(&msg, &content_hash);
        let rel_path = state
            .storage
            .write_message(
                &msg.source,
                &msg.conversation_id,
                &msg.id,
                msg.timestamp.timestamp(),
                &content_hash,
                &eml_content,
            )
            .map_err(|e| {
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Failed to write EML: {}", e),
                )
            })?;
        rel_path_str = rel_path.to_string_lossy().to_string();
    }

    // Index to SQLite (if enabled)
    if state.config.storage.enable_db {
        let mut db = state.db.lock().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
        })?;
        index_message(
            db.conn_mut(),
            &msg,
            &content_hash,
            &rel_path_str,
        )
        .map_err(|e| {
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to index message: {}", e),
            )
        })?;
    }

    Ok(Json(serde_json::json!({
        "status": "indexed",
        "message_id": msg.id,
        "content_hash": content_hash,
        "eml_path": rel_path_str
    })))
}
