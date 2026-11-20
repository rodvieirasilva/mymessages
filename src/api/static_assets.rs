use axum::body::Body;
use axum::http::{header, Request, Response, StatusCode};
use rust_embed::RustEmbed;

#[derive(RustEmbed)]
#[folder = "web/dist"]
pub struct EmbeddedWebAssets;

pub async fn embedded_static_handler(req: Request<Body>) -> Response<Body> {
    let raw_path = req.uri().path().trim_start_matches('/');
    let path = if raw_path.is_empty() { "index.html" } else { raw_path };

    let asset = EmbeddedWebAssets::get(path).or_else(|| EmbeddedWebAssets::get("index.html"));
    let Some(file) = asset else {
        return Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Body::from("Not Found"))
            .unwrap();
    };

    let mime = mime_for_path(if EmbeddedWebAssets::get(path).is_some() {
        path
    } else {
        "index.html"
    });

    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, mime)
        .body(Body::from(file.data.into_owned()))
        .unwrap()
}

fn mime_for_path(path: &str) -> &'static str {
    let ext = path.rsplit('.').next().unwrap_or("").to_lowercase();
    match ext.as_str() {
        "html" => "text/html; charset=utf-8",
        "js" | "mjs" => "application/javascript; charset=utf-8",
        "css" => "text/css; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "ico" => "image/x-icon",
        "json" => "application/json",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_embedded_static_handler_serves_index() {
        let req = Request::builder().uri("/").body(Body::empty()).unwrap();
        let resp = embedded_static_handler(req).await;
        assert_eq!(resp.status(), StatusCode::OK);
        assert_eq!(
            resp.headers().get(header::CONTENT_TYPE).unwrap(),
            "text/html; charset=utf-8"
        );
    }
}
