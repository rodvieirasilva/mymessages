use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::index::query_dsl::CompiledQuery;

#[derive(Debug, Clone, Default, Deserialize, ToSchema)]
pub struct SearchQuery {
    #[serde(default)]
    pub query: String,
    pub source: Option<String>,
    pub chat_id: Option<String>,
    pub has_media: Option<bool>,
    pub media_type: Option<String>,
    pub is_from_me: Option<bool>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

impl SearchQuery {
    pub fn to_compiled(&self) -> CompiledQuery {
        let mut compiled = CompiledQuery {
            source: self.source.clone(),
            chat_id: self.chat_id.clone(),
            has_media: self.has_media,
            media_type: self.media_type.clone(),
            is_from_me: self.is_from_me,
            limit: self.limit.unwrap_or(50).clamp(1, 100),
            offset: self.offset.unwrap_or(0),
            ..Default::default()
        };
        crate::index::lucene::parse_lucene_string(&self.query, &mut compiled);
        compiled
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AttachmentInfo {
    pub id: String,
    pub filename: Option<String>,
    pub mimetype: String,
    pub file_size: i64,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SearchResult {
    pub id: String,
    pub source: String,
    pub conversation_id: String,
    pub conversation_name: Option<String>,
    pub sender_name: Option<String>,
    pub timestamp: i64,
    pub body: Option<String>,
    pub eml_path: String,
    pub has_attachments: bool,
    pub is_from_me: bool,
    pub snippet: Option<String>,
    pub attachments: Vec<AttachmentInfo>,
}
