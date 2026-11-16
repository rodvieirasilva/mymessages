use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::api::state::AppState;
use crate::index::{search_messages, SearchQuery};

pub async fn search_handler(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
    })?;

    let results = search_messages(db.conn(), &params).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Search failed: {}", e))
    })?;

    Ok(Json(results))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::index::Database;

    #[tokio::test]
    async fn test_search_handler_empty() {
        let db = Database::in_memory().unwrap();
        let state = AppState::new(db, std::env::temp_dir(), AppConfig::default());
        let query = SearchQuery {
            query: "rust".into(),
            ..Default::default()
        };

        let resp = search_handler(State(state), Query(query)).await.unwrap();
        let json = resp.into_response();
        assert_eq!(json.status(), StatusCode::OK);
    }
}
