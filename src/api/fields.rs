use axum::response::IntoResponse;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldInfo {
    pub name: &'static str,
    pub lucene_key: &'static str,
    pub kind: &'static str,
    pub fts_indexed: bool,
    pub facetable: bool,
    pub wildcard_supported: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct FieldsResponse {
    pub fields: Vec<FieldInfo>,
}

#[utoipa::path(
    get,
    path = "/v1/fields",
    tag = "search",
    responses(
        (status = 200, description = "List of all indexed and facetable fields. Keyword/text fields support both exact values and SQLite LIKE wildcards (% or *).", body = FieldsResponse)
    )
)]
pub async fn fields_handler() -> impl IntoResponse {
    Json(FieldsResponse {
        fields: vec![
            FieldInfo { name: "body", lucene_key: "body", kind: "text", fts_indexed: true, facetable: false, wildcard_supported: true },
            FieldInfo { name: "conversation_name", lucene_key: "chat", kind: "text+keyword", fts_indexed: true, facetable: true, wildcard_supported: true },
            FieldInfo { name: "sender_name", lucene_key: "from", kind: "text+keyword", fts_indexed: true, facetable: true, wildcard_supported: true },
            FieldInfo { name: "source_id", lucene_key: "source", kind: "keyword", fts_indexed: false, facetable: true, wildcard_supported: true },
            FieldInfo { name: "account_id", lucene_key: "account", kind: "keyword", fts_indexed: false, facetable: true, wildcard_supported: true },
            FieldInfo { name: "media_type", lucene_key: "media_type", kind: "keyword", fts_indexed: false, facetable: true, wildcard_supported: true },
            FieldInfo { name: "has_attachments", lucene_key: "has_media", kind: "boolean", fts_indexed: false, facetable: true, wildcard_supported: false },
            FieldInfo { name: "is_from_me", lucene_key: "is_from_me", kind: "boolean", fts_indexed: false, facetable: true, wildcard_supported: false },
            FieldInfo { name: "attachment_filenames", lucene_key: "filename", kind: "text", fts_indexed: true, facetable: false, wildcard_supported: true },
            FieldInfo { name: "metadata_text", lucene_key: "meta", kind: "text", fts_indexed: true, facetable: true, wildcard_supported: true },
            FieldInfo { name: "timestamp", lucene_key: "after/before", kind: "date", fts_indexed: false, facetable: true, wildcard_supported: false },
        ],
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::http::StatusCode;

    #[tokio::test]
    async fn test_fields_handler() {
        let resp = fields_handler().await.into_response();
        assert_eq!(resp.status(), StatusCode::OK);
    }
}
