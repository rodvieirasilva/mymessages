use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::state::AppState;
use crate::index::{get_chat_messages, list_chats};

#[derive(Debug, Deserialize)]
pub struct ChatMessagesQuery {
    pub limit: Option<usize>,
    pub before: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ChatsFilter {
    pub source: Option<String>,
}

pub async fn list_chats_handler(
    State(state): State<AppState>,
    Query(params): Query<ChatsFilter>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
    })?;

    let chats = list_chats(db.conn(), params.source.as_deref()).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list chats: {}", e))
    })?;

    Ok(Json(chats))
}

pub async fn get_chat_messages_handler(
    State(state): State<AppState>,
    Path(chat_id): Path<String>,
    Query(params): Query<ChatMessagesQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
    })?;

    let limit = params.limit.unwrap_or(50);
    let messages = get_chat_messages(db.conn(), &chat_id, limit, params.before).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to get chat messages: {}", e))
    })?;

    Ok(Json(messages))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::index::Database;

    #[tokio::test]
    async fn test_list_and_get_chat_messages_handler() {
        let db = Database::in_memory().unwrap();
        let state = AppState::new(db, std::env::temp_dir(), AppConfig::default());

        let resp_list = list_chats_handler(State(state.clone()), Query(ChatsFilter { source: None }))
            .await
            .unwrap();
        assert_eq!(resp_list.into_response().status(), StatusCode::OK);

        let resp_msgs = get_chat_messages_handler(
            State(state),
            Path("chat_123".into()),
            Query(ChatMessagesQuery { limit: Some(10), before: None }),
        )
        .await
        .unwrap();
        assert_eq!(resp_msgs.into_response().status(), StatusCode::OK);
    }
}
