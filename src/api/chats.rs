use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Deserialize;

use crate::api::state::AppState;
use crate::index::{get_chat_messages, list_chats, ChatSummary, SearchResult};

#[derive(Debug, Deserialize)]
pub struct ChatMessagesQuery {
    pub limit: Option<usize>,
    pub before: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct ChatsFilter {
    pub source: Option<String>,
    pub query: Option<String>,
}

#[utoipa::path(
    get,
    path = "/v1/chats",
    tag = "chats",
    params(
        ("source" = Option<String>, Query, description = "Filter conversations by source ID (e.g. whatsapp)"),
        ("query" = Option<String>, Query, description = "Filter conversations by name, remote ID, or message text")
    ),
    responses(
        (status = 200, description = "List of conversation summaries", body = Vec<ChatSummary>),
        (status = 500, description = "Database error")
    )
)]
pub async fn list_chats_handler(
    State(state): State<AppState>,
    Query(params): Query<ChatsFilter>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
    })?;

    let chats = list_chats(
        db.conn(),
        params.source.as_deref(),
        params.query.as_deref(),
    ).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Failed to list chats: {}", e))
    })?;

    Ok(Json(chats))
}

#[utoipa::path(
    get,
    path = "/v1/chats/{id}/messages",
    tag = "chats",
    params(
        ("id" = String, Path, description = "Conversation unique ID"),
        ("limit" = Option<usize>, Query, description = "Maximum messages to return (default 50)"),
        ("before" = Option<i64>, Query, description = "Timestamp pagination cursor"),
    ),
    responses(
        (status = 200, description = "List of messages in conversation", body = Vec<SearchResult>),
        (status = 500, description = "Database error")
    )
)]
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

        let resp_list = list_chats_handler(
            State(state.clone()),
            Query(ChatsFilter { source: None, query: None }),
        )
        .await
        .unwrap();
        assert_eq!(resp_list.into_response().status(), StatusCode::OK);

        let resp_list_filter = list_chats_handler(
            State(state.clone()),
            Query(ChatsFilter { source: Some("whatsapp".into()), query: Some("test".into()) }),
        )
        .await
        .unwrap();
        assert_eq!(resp_list_filter.into_response().status(), StatusCode::OK);

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
