use axum::extract::{Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, StatusCode};
use axum::response::IntoResponse;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::fs;

use crate::api::state::AppState;

#[utoipa::path(
    get,
    path = "/api/v1/attachments/{id}",
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

    let full_eml_path = if state.storage.root_dir.join(&eml_rel_path).exists() {
        state.storage.root_dir.join(&eml_rel_path)
    } else {
        state.data_dir.join(&eml_rel_path)
    };
    let eml_content = fs::read_to_string(&full_eml_path).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read EML at {:?}: {}", full_eml_path, e),
        )
    })?;

    // Extract base64 part with matching SHA-256 header
    let target_header = format!("X-Attachment-SHA256: {}", sha256);
    let pos = eml_content
        .find(&target_header)
        .ok_or((StatusCode::NOT_FOUND, "Attachment part not found in EML".into()))?;

    let after_header = &eml_content[pos + target_header.len()..];
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
    headers.insert(CONTENT_TYPE, mimetype.parse().unwrap());
    if let Some(fname) = filename {
        headers.insert(
            CONTENT_DISPOSITION,
            format!("inline; filename=\"{}\"", fname).parse().unwrap(),
        );
    }

    Ok((headers, binary))
}
