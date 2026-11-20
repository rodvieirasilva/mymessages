use axum::http::StatusCode;
use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct HealthResponse {
    pub status: &'static str,
    pub version: &'static str,
    pub build_time: &'static str,
    pub commit: &'static str,
}

#[utoipa::path(
    get,
    path = "/v1/health",
    tag = "health",
    responses(
        (status = 200, description = "Service is healthy")
    )
)]
pub async fn health_handler() -> impl IntoResponse {
    (
        StatusCode::OK,
        Json(HealthResponse {
            status: "ok",
            version: env!("CARGO_PKG_VERSION"),
            build_time: env!("MYMESSAGES_BUILD_TIME"),
            commit: env!("MYMESSAGES_GIT_SHA"),
        }),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::to_bytes;

    #[tokio::test]
    async fn test_health_handler() {
        let resp = health_handler().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
        let body = to_bytes(resp.into_body(), usize::MAX).await.unwrap();
        let json: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(json["status"], "ok");
        assert!(!json["version"].as_str().unwrap_or("").is_empty());
        assert!(!json["build_time"].as_str().unwrap_or("").is_empty());
        assert!(!json["commit"].as_str().unwrap_or("").is_empty());
    }
}
