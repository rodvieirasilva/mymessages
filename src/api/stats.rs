use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use crate::api::state::{AppState, IngestionStatus};
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
    pub ingestion: IngestionStatus,
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

    let ingestion = state
        .ingestion
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default();

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
            ingestion,
        })
        .into_response(),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()).into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::index::Database;

    #[tokio::test]
    async fn test_stats_handler() {
        let dir = std::env::temp_dir().join("mymessages_test_api_stats");
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let db_path = dir.join("messages.sqlite3");
        let db = Database::open(&db_path).unwrap();
        let state = AppState::new(db, dir.clone(), AppConfig::default());

        let resp = stats_handler(State(state)).await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
