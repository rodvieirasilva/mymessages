use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::api::state::AppState;
use crate::mcp::protocol::{JsonRpcRequest, JsonRpcResponse};
use crate::mcp::server::handle_request;

#[utoipa::path(
    post,
    path = "/v1/mcp",
    tag = "mcp",
    request_body = JsonRpcRequest,
    responses(
        (status = 200, description = "JSON-RPC 2.0 response for MCP tools", body = JsonRpcResponse),
        (status = 204, description = "Notification handled (no content)"),
        (status = 500, description = "Database lock error")
    )
)]
pub async fn mcp_http_handler(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
    })?;

    if let Some(resp) = handle_request(&db, &req) {
        Ok(Json(resp).into_response())
    } else {
        Ok(StatusCode::NO_CONTENT.into_response())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::index::Database;
    use serde_json::json;

    #[tokio::test]
    async fn test_mcp_http_handler() {
        let db = Database::in_memory().unwrap();
        let state = AppState::new(db, std::env::temp_dir(), AppConfig::default());

        let req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: json!({}),
        };

        let resp = mcp_http_handler(State(state.clone()), Json(req)).await.unwrap().into_response();
        assert_eq!(resp.status(), StatusCode::OK);

        // Test tools/list
        let list_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "tools/list".into(),
            params: json!({}),
        };
        let resp2 = mcp_http_handler(State(state.clone()), Json(list_req)).await.unwrap().into_response();
        assert_eq!(resp2.status(), StatusCode::OK);

        // Test notifications (id = None)
        let notif_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: None,
            method: "notifications/initialized".into(),
            params: json!({}),
        };
        let resp3 = mcp_http_handler(State(state), Json(notif_req)).await.unwrap().into_response();
        assert_eq!(resp3.status(), StatusCode::NO_CONTENT);
    }
}
