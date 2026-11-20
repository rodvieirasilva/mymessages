use axum::extract::{Path, State};
use axum::http::header::{CONTENT_DISPOSITION, CONTENT_TYPE};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::response::IntoResponse;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use std::fs;
use std::path::Path as StdPath;

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
        (status = 403, description = "External attachment outside configured media roots"),
        (status = 404, description = "Attachment or EML file not found"),
        (status = 500, description = "Internal error extracting attachment")
    )
)]
pub async fn get_attachment_handler(
    State(state): State<AppState>,
    Path(att_id): Path<String>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let (eml_rel_path, mimetype, filename, sha256, external_path) = {
        let db = state.db.lock().map_err(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, format!("DB lock error: {}", e))
        })?;

        let sql = "
            SELECT m.eml_path, a.mimetype, a.filename, a.sha256, a.external_path
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
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(|_| (StatusCode::NOT_FOUND, "Attachment not found".into()))?
    };

    let binary = if let Some(ext_path_str) = external_path {
        read_external_attachment(&state, StdPath::new(&ext_path_str))?
    } else {
        read_eml_attachment(&state, &eml_rel_path, &sha256)?
    };

    let headers = build_attachment_headers(&mimetype, filename);
    Ok((headers, binary))
}

fn read_external_attachment(
    state: &AppState,
    ext_path: &StdPath,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let canonical_target = fs::canonicalize(ext_path)
        .map_err(|_| (StatusCode::NOT_FOUND, "External attachment file not found".into()))?;

    let mut allowed_roots = state.config.allowed_media_roots();
    allowed_roots.push(state.data_dir.clone());
    allowed_roots.push(state.storage.root_dir.clone());

    let is_allowed = allowed_roots.iter().any(|root| {
        fs::canonicalize(root)
            .map(|canon_root| canonical_target.starts_with(&canon_root))
            .unwrap_or(false)
    });

    if !is_allowed {
        return Err((
            StatusCode::FORBIDDEN,
            "External attachment is outside configured media roots".into(),
        ));
    }

    fs::read(&canonical_target).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to read external attachment: {}", e),
        )
    })
}

fn read_eml_attachment(
    state: &AppState,
    eml_rel_path: &str,
    sha256: &str,
) -> Result<Vec<u8>, (StatusCode, String)> {
    let rel_path = StdPath::new(eml_rel_path);
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

    let header_end = eml_content
        .find("\r\n\r\n")
        .or_else(|| eml_content.find("\n\n"))
        .unwrap_or(0);
    let mime_body = &eml_content[header_end..];

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

    BASE64.decode(raw_b64).map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Base64 decode failed: {}", e),
        )
    })
}

fn build_attachment_headers(mimetype: &str, filename: Option<String>) -> HeaderMap {
    let mut headers = HeaderMap::new();
    let content_type = HeaderValue::from_str(mimetype)
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

    headers
}
