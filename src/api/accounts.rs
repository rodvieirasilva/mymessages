use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;

use crate::api::state::AppState;
use crate::index::{list_accounts, AccountSummary};

#[utoipa::path(
    get,
    path = "/v1/accounts",
    tag = "accounts",
    responses(
        (status = 200, description = "List of indexed accounts with message and chat counts", body = Vec<AccountSummary>),
        (status = 500, description = "Database error")
    )
)]
pub async fn list_accounts_handler(
    State(state): State<AppState>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
    })?;

    let accounts = list_accounts(db.conn()).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list accounts: {}", e))
    })?;

    Ok(Json(accounts))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::index::Database;

    #[tokio::test]
    async fn test_list_accounts_handler() {
        let db = Database::in_memory().unwrap();
        let state = AppState::new(db, std::env::temp_dir(), AppConfig::default());
        let resp = list_accounts_handler(State(state)).await.unwrap();
        assert_eq!(resp.into_response().status(), StatusCode::OK);
    }
}
