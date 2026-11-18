use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde_json::json;

#[utoipa::path(
    get,
    path = "/v1/health",
    tag = "system",
    responses(
        (status = 200, description = "System health check response")
    )
)]
pub async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(json!({
            "status": "ok",
            "version": env!("CARGO_PKG_VERSION")
        })),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_health_handler() {
        let resp = health_handler().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
