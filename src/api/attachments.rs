use axum::extract::{Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::fs;

use crate::api::state::AppState;

#[utoipa::path(
    get,
    path = "/v1/attachments/{id}",
    tag = "attachments",
    params(
        ("id" = String, Path, description = "Unique attachment identifier"),
    ),
    responses(
        (status = 200, description = "Streamed binary attachment content", content_type = "application/octet-stream"),
        (status = 404, description = "Attachment or EML file not found"),
        (status = 500, description = "Internal error extracting attachment")
    )
)]
pub async fn get_attachment_handler(
    State(state): State<AppState>,
    Path(att_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (eml_rel_path, mimetype, filename, sha256) = {
        let db = state.db.lock().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
        })?;

        let sql = "
            SELECT m.eml_path, a.mimetype, a.filename, a.sha256
            FROM attachments a
            JOIN messages m ON m.id = a.message_id
            WHERE a.id = ?1
        ";
        let mut stmt = db.conn().prepare(sql).map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("Prepare error: {}", e))
        })?;

        stmt.query_row([&att_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, String>(3)?,
            ))
        })
        .map_err(|_| (StatusCode::NOT_FOUND, "Attachment not found".into()))?
    };

    let rel_path = std::path::Path::new(&eml_rel_path);
    if rel_path.is_absolute()
        || rel_path.components().any(|c| {
            matches!(
                c,
                std::path::Component::ParentDir
                    | std::path::Component::RootDir
                    | std::path::Component::Prefix(_)
            )
        })
    {
        return Err((StatusCode::BAD_REQUEST, "Invalid attachment path".into()));
    }

    let full_eml_path = if state.storage.root_dir.join(rel_path).exists() {
        state.storage.root_dir.join(rel_path)
    } else {
        state.data_dir.join(rel_path)
    };
    let eml_content = fs::read_to_string(&full_eml_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read EML at {:?}: {}", full_eml_path, e),
        )
    })?;

    // Find the header boundary to avoid matching headers inside message text
    let header_end = eml_content
        .find("\r\n\r\n")
        .or_else(|| eml_content.find("\n\n"))
        .unwrap_or(0);
    let mime_body = &eml_content[header_end..];

    // Extract base64 part with matching SHA-256 header
    let target_header = format!("X-Attachment-SHA256: {}", sha256);
    let pos = mime_body
        .find(&target_header)
        .ok_or((StatusCode::NOT_FOUND, "Attachment part not found in EML".into()))?;

    let after_header = &mime_body[pos + target_header.len()..];
    let body_start = after_header
        .find("\r\n\r\n")
        .map(|i| i + 4)
        .or_else(|| after_header.find("\n\n").map(|i| i + 2))
        .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Invalid MIME format in EML".into()))?;

    let body_slice = &after_header[body_start..];
    let end_pos = body_slice
        .find("\r\n--")
        .or_else(|| body_slice.find("\n--"))
        .unwrap_or(body_slice.len());

    let raw_b64: String = body_slice[..end_pos]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();

    let binary = BASE64.decode(raw_b64).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Base64 decode failed: {}", e),
        )
    })?;

    let mut headers = HeaderMap::new();
    let content_type = HeaderValue::from_str(&mimetype)
        .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream"));
    headers.insert(CONTENT_TYPE, content_type);
    headers.insert(
        axum::http::header::HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );

    let is_safe_inline = (mimetype.starts_with("image/") && mimetype != "image/svg+xml")
        || mimetype.starts_with("audio/")
        || mimetype.starts_with("video/")
        || mimetype == "application/pdf";

    let disposition_type = if is_safe_inline { "inline" } else { "attachment" };
    if let Some(fname) = filename {
        let clean_fname: String = fname
            .chars()
            .filter(|c| !c.is_control() && *c != '"' && *c != '\r' && *c != '\n')
            .collect();
        let disp_val = format!("{}; filename=\"{}\"", disposition_type, clean_fname);
        if let Ok(hv) = HeaderValue::from_str(&disp_val) {
            headers.insert(CONTENT_DISPOSITION, hv);
        } else if let Ok(hv) = HeaderValue::from_str(disposition_type) {
            headers.insert(CONTENT_DISPOSITION, hv);
        }
    } else if let Ok(hv) = HeaderValue::from_str(disposition_type) {
        headers.insert(CONTENT_DISPOSITION, hv);
    }

    Ok((headers, binary))
}
