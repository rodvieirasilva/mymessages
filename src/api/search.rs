use axum::body::Bytes;
use axum::extract::{Query, State};
use axum::http::{Method, StatusCode};
use axum::response::IntoResponse;
use axum::Json;

use crate::api::state::AppState;
use crate::index::{search_compiled, SearchQuery, SearchRequest, SearchResult};

#[utoipa::path(
    get,
    path = "/v1/search",
    tag = "search",
    params(
        ("query" = Option<String>, Query, description = "Lucene query string (e.g. 'contract from:Alice after:2024-01-01')"),
        ("source" = Option<String>, Query, description = "Filter by source (e.g. whatsapp)"),
        ("chat_id" = Option<String>, Query, description = "Filter by conversation ID"),
        ("has_media" = Option<bool>, Query, description = "Filter messages that contain attachments"),
        ("media_type" = Option<String>, Query, description = "Filter by media type (image, audio, video, document)"),
        ("is_from_me" = Option<bool>, Query, description = "Filter by outgoing (true) or incoming (false)"),
        ("limit" = Option<usize>, Query, description = "Maximum number of messages to return (default 50)"),
        ("offset" = Option<usize>, Query, description = "Offset for pagination (default 0)"),
    ),
    responses(
        (status = 200, description = "Matching search results. Supports GET with params, or QUERY/POST with JSON Query DSL body.", body = Vec<SearchResult>),
        (status = 400, description = "Malformed JSON query body"),
        (status = 500, description = "Internal database search error")
    )
)]
pub async fn search_unified_handler(
    method: Method,
    State(state): State<AppState>,
    Query(get_params): Query<SearchQuery>,
    body_bytes: Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let db = state.db.lock().map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
    })?;

    let compiled = if method.as_str() == "QUERY" || method == Method::POST {
        if !body_bytes.is_empty() {
            let req: SearchRequest = serde_json::from_slice(&body_bytes).map_err(|e| {
                (StatusCode::BAD_REQUEST, format!("Invalid JSON query body: {}", e))
            })?;
            req.compile()
        } else {
            get_params.to_compiled()
        }
    } else if method == Method::GET {
        get_params.to_compiled()
    } else {
        return Err((StatusCode::METHOD_NOT_ALLOWED, "Method not allowed. Use GET, POST, or QUERY.".into()));
    };

    let results = search_compiled(db.conn(), &compiled).map_err(|e| {
        (StatusCode::INTERNAL_SERVER_ERROR, format!("Search failed: {}", e))
    })?;

    Ok(Json(results))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::AppConfig;
    use crate::index::Database;
    use serde_json::json;

    #[tokio::test]
    async fn test_search_handler_get_post_and_query() {
        let db = Database::in_memory().unwrap();
        let state = AppState::new(db, std::env::temp_dir(), AppConfig::default());

        // 1. GET
        let query = SearchQuery {
            query: "rust".into(),
            ..Default::default()
        };
        let resp_get = search_unified_handler(Method::GET, State(state.clone()), Query(query), Bytes::new()).await.unwrap();
        assert_eq!(resp_get.into_response().status(), StatusCode::OK);

        // 2. QUERY method with Elasticsearch JSON body
        let query_method = Method::from_bytes(b"QUERY").unwrap();
        let json_body = json!({
            "query": {
                "bool": {
                    "must": [{ "match": { "body": "rust" } }],
                    "filter": [{ "term": { "source": "whatsapp" } }]
                }
            }
        });
        let body_bytes = Bytes::from(serde_json::to_vec(&json_body).unwrap());
        let resp_query = search_unified_handler(query_method, State(state.clone()), Query(SearchQuery::default()), body_bytes).await.unwrap();
        assert_eq!(resp_query.into_response().status(), StatusCode::OK);

        // 3. POST method with JSON body
        let post_body = Bytes::from(serde_json::to_vec(&json!({ "query_string": "test from:Alice" })).unwrap());
        let resp_post = search_unified_handler(Method::POST, State(state.clone()), Query(SearchQuery::default()), post_body).await.unwrap();
        assert_eq!(resp_post.into_response().status(), StatusCode::OK);

        // 4. Invalid method
        let resp_err = search_unified_handler(Method::DELETE, State(state), Query(SearchQuery::default()), Bytes::new()).await;
        assert_eq!(resp_err.err().unwrap().0, StatusCode::METHOD_NOT_ALLOWED);
    }
}
