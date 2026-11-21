use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::state::AppState;
use crate::index::{get_vault_stats, SourceStats};

#[derive(Debug, Serialize, ToSchema)]
pub struct StatsResponse {
    pub version: &'static str,
    pub build_time: &'static str,
    pub commit: &'static str,
    pub db_path: String,
    pub db_size_bytes: u64,
    pub total_messages: i64,
    pub total_chats: i64,
    pub sources: Vec<SourceStats>,
}

#[utoipa::path(
    get,
    path = "/v1/stats",
    tag = "stats",
    responses(
        (status = 200, description = "Vault statistics and database size", body = StatsResponse)
    )
)]
pub async fn stats_handler(State(state): State<AppState>) -> impl IntoResponse {
    let guard = match state.db.lock() {
        Ok(g) => g,
        Err(_) => return (StatusCode::INTERNAL_SERVER_ERROR, "DB lock failed").into_response(),
    };

    match get_vault_stats(guard.conn(), &state.db_path) {
        Ok(stats) => Json(StatsResponse {
            version: env!("CARGO_PKG_VERSION"),
            build_time: option_env!("MYMESSAGES_BUILD_TIME").unwrap_or("unknown"),
            commit: option_env!("MYMESSAGES_GIT_SHA").unwrap_or("unknown"),
            db_path: stats.db_path,
            db_size_bytes: stats.db_size_bytes,
            total_messages: stats.total_messages,
            total_chats: stats.total_chats,
            sources: stats.sources,
        })
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}
