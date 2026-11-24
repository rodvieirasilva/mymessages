use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::api::fields::{FieldInfo, FieldsResponse};
use crate::api::state::IngestionStatus;
use crate::api::stats::StatsResponse;
use crate::api::webhook::WebhookResult;
use crate::index::{AccountSummary, AttachmentInfo, ChatSummary, SearchQuery, SearchRequest, SearchResult, SourceStats};
use crate::mcp::protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};

#[derive(OpenApi)]
#[openapi(
    paths(
        crate::api::accounts::list_accounts_handler,
        crate::api::search::search_unified_handler,
        crate::api::chats::list_chats_handler,
        crate::api::chats::get_chat_messages_handler,
        crate::api::attachments::get_attachment_handler,
        crate::api::mcp::mcp_http_handler,
        crate::api::webhook::evolution_webhook_handler,
        crate::api::webhook::waha_webhook_handler,
        crate::api::health::health_handler,
        crate::api::stats::stats_handler,
        crate::api::fields::fields_handler,
    ),
    components(
        schemas(
            AccountSummary,
            SearchResult,
            ChatSummary,
            AttachmentInfo,
            SearchQuery,
            SearchRequest,
            JsonRpcRequest,
            JsonRpcResponse,
            JsonRpcError,
            WebhookResult,
            StatsResponse,
            SourceStats,
            IngestionStatus,
            FieldsResponse,
            FieldInfo,
        )
    ),
    tags(
        (name = "accounts", description = "Multi-account discovery and facet summaries"),
        (name = "search", description = "Full-text message search and filtering"),
        (name = "chats", description = "Conversation summaries and chat histories"),
        (name = "attachments", description = "Media streaming and attachment retrieval"),
        (name = "mcp", description = "Model Context Protocol JSON-RPC 2.0 endpoint"),
        (name = "webhook", description = "External messaging platform ingestion endpoints"),
    ),
    info(
        title = "mymessages API",
        version = "0.3.0",
        description = "Local-first message vault, Maildir archiver, and SQLite FTS5 search engine"
    )
)]
pub struct ApiDoc;

pub fn swagger_ui() -> SwaggerUi {
    SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_generation() {
        let doc = ApiDoc::openapi();
        let json_str = doc.to_json().unwrap();
        assert!(json_str.contains("mymessages API"));
        assert!(json_str.contains("/v1/search"));
        assert!(json_str.contains("/v1/chats"));
        assert!(json_str.contains("/v1/chats/{id}/messages"));
        assert!(json_str.contains("/v1/attachments/{id}"));
        assert!(json_str.contains("/v1/mcp"));
        assert!(json_str.contains("/v1/webhook/evolution"));
        assert!(json_str.contains("/v1/webhook/waha"));
    }
}
