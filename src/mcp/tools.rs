use serde_json::{json, Value};
use crate::index::{get_chat_messages, list_chats, search_messages, Database, SearchQuery};
use super::protocol::{McpCallToolResult, McpTool};

pub fn get_available_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "search_messages".into(),
            description: "Search messages in the local vault using SQLite FTS5 full-text search. Returns matching messages with relevance snippets, sender, timestamp, and attachments.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword or phrase" },
                    "chat_id": { "type": "string", "description": "Optional conversation ID to filter by" },
                    "source": { "type": "string", "description": "Optional source filter (e.g. 'whatsapp')" },
                    "has_media": { "type": "boolean", "description": "Filter for messages with attachments" },
                    "media_type": { "type": "string", "description": "Filter by media type: 'image', 'audio', 'document', 'video'" },
                    "limit": { "type": "integer", "description": "Max results to return (default 20, max 100)" }
                },
                "required": ["query"]
            }),
        },
        McpTool {
            name: "list_chats".into(),
            description: "List conversations and group chats in the vault with metadata including participant names, message counts, and last activity.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Optional source filter (e.g. 'whatsapp')" }
                }
            }),
        },
        McpTool {
            name: "get_chat_history".into(),
            description: "Retrieve chronological message history for a specific conversation ID.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "chat_id": { "type": "string", "description": "Conversation unique ID" },
                    "limit": { "type": "integer", "description": "Max messages to retrieve (default 30)" },
                    "before": { "type": "integer", "description": "Timestamp cursor to paginate backwards" }
                },
                "required": ["chat_id"]
            }),
        },
        McpTool {
            name: "get_attachment_info".into(),
            description: "Retrieve metadata, MIME type, SHA-256 hash, and Maildir EML path for an attachment.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "attachment_id": { "type": "string", "description": "Unique attachment ID" }
                },
                "required": ["attachment_id"]
            }),
        },
    ]
}

pub fn execute_tool(db: &Database, name: &str, args: &Value) -> McpCallToolResult {
    match name {
        "search_messages" => {
            let query = args.get("query").and_then(|q| q.as_str()).unwrap_or("").to_string();
            let chat_id = args.get("chat_id").and_then(|c| c.as_str()).map(|s| s.to_string());
            let source = args.get("source").and_then(|s| s.as_str()).map(|s| s.to_string());
            let has_media = args.get("has_media").and_then(|h| h.as_bool());
            let media_type = args.get("media_type").and_then(|m| m.as_str()).map(|s| s.to_string());
            let is_from_me = args.get("is_from_me").and_then(|f| f.as_bool());
            let limit = args.get("limit").and_then(|l| l.as_u64()).map(|u| u as usize).unwrap_or(20);

            let sq = SearchQuery {
                query,
                source,
                chat_id,
                has_media,
                media_type,
                is_from_me,
                limit: Some(limit),
                offset: Some(0),
            };

            match search_messages(db.conn(), &sq) {
                Ok(results) => {
                    let json_text = serde_json::to_string_pretty(&results).unwrap_or_default();
                    McpCallToolResult::text(format!("Found {} messages:\n{}", results.len(), json_text))
                }
                Err(e) => McpCallToolResult::error(format!("Search failed: {}", e)),
            }
        }

        "list_chats" => {
            let source = args.get("source").and_then(|s| s.as_str());
            match list_chats(db.conn(), source) {
                Ok(chats) => {
                    let json_text = serde_json::to_string_pretty(&chats).unwrap_or_default();
                    McpCallToolResult::text(format!("Retrieved {} conversations:\n{}", chats.len(), json_text))
                }
                Err(e) => McpCallToolResult::error(format!("Failed to list chats: {}", e)),
            }
        }

        "get_chat_history" => {
            let chat_id = match args.get("chat_id").and_then(|c| c.as_str()) {
                Some(id) => id,
                None => return McpCallToolResult::error("Missing required parameter: chat_id"),
            };
            let limit = args.get("limit").and_then(|l| l.as_u64()).map(|u| u as usize).unwrap_or(30);
            let before = args.get("before").and_then(|b| b.as_i64());

            match get_chat_messages(db.conn(), chat_id, limit, before) {
                Ok(messages) => {
                    let json_text = serde_json::to_string_pretty(&messages).unwrap_or_default();
                    McpCallToolResult::text(format!("Retrieved {} messages for chat '{}':\n{}", messages.len(), chat_id, json_text))
                }
                Err(e) => McpCallToolResult::error(format!("Failed to retrieve chat messages: {}", e)),
            }
        }

        "get_attachment_info" => {
            let att_id = match args.get("attachment_id").and_then(|a| a.as_str()) {
                Some(id) => id,
                None => return McpCallToolResult::error("Missing required parameter: attachment_id"),
            };

            let sql = "
                SELECT a.id, a.message_id, a.mimetype, a.filename, a.file_size, a.sha256, a.media_type, m.eml_path
                FROM attachments a
                JOIN messages m ON m.id = a.message_id
                WHERE a.id = ?1
            ";
            let mut stmt = match db.conn().prepare(sql) {
                Ok(s) => s,
                Err(e) => return McpCallToolResult::error(format!("DB query error: {}", e)),
            };

            let res = stmt.query_row([att_id], |row| {
                Ok(json!({
                    "id": row.get::<_, String>(0)?,
                    "message_id": row.get::<_, String>(1)?,
                    "mimetype": row.get::<_, String>(2)?,
                    "filename": row.get::<_, Option<String>>(3)?,
                    "file_size": row.get::<_, i64>(4)?,
                    "sha256": row.get::<_, String>(5)?,
                    "media_type": row.get::<_, String>(6)?,
                    "eml_path": row.get::<_, String>(7)?,
                }))
            });

            match res {
                Ok(info) => McpCallToolResult::text(serde_json::to_string_pretty(&info).unwrap_or_default()),
                Err(_) => McpCallToolResult::error(format!("Attachment '{}' not found", att_id)),
            }
        }

        _ => McpCallToolResult::error(format!("Unknown tool: {}", name)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::index::writer::index_message;
    use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};
    use chrono::Utc;

    fn setup_test_db() -> Database {
        let mut db = Database::in_memory().unwrap();
        let att = CanonicalAttachment::new(
            Some("doc.pdf".into()),
            "application/pdf".into(),
            b"pdfdata".to_vec(),
        );
        let msg = CanonicalMessage {
            id: "msg_mcp_1".into(),
            source: "whatsapp".into(),
            conversation_id: "chat_mcp_1".into(),
            conversation_name: Some("Project MCP".into()),
            is_group: false,
            sender: CanonicalParticipant {
                id: "sender_1".into(),
                name: Some("Dave".into()),
                phone_number: Some("+12345".into()),
            },
            recipients: vec![],
            timestamp: Utc::now(),
            body: Some("Here is the contract document for our project".into()),
            attachments: vec![att],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: json!({}),
        };
        index_message(db.conn_mut(), &msg, "hash_mcp_1", "mail/wh/cur/1.eml").unwrap();
        db
    }

    #[test]
    fn test_execute_mcp_tools() {
        let db = setup_test_db();

        // 1. search_messages
        let res = execute_tool(&db, "search_messages", &json!({ "query": "contract" }));
        assert!(!res.is_error);
        assert!(res.content[0].text.contains("Found 1 messages"));

        // 2. list_chats
        let res_chats = execute_tool(&db, "list_chats", &json!({}));
        assert!(!res_chats.is_error);
        assert!(res_chats.content[0].text.contains("Retrieved 1 conversations"));

        // 3. get_chat_history
        let res_hist = execute_tool(
            &db,
            "get_chat_history",
            &json!({ "chat_id": "whatsapp:chat_mcp_1" }),
        );
        assert!(!res_hist.is_error);
        assert!(res_hist.content[0].text.contains("Retrieved 1 messages"));

        // 4. get_attachment_info
        let att_id: String = db.conn().query_row("SELECT id FROM attachments LIMIT 1", [], |r| r.get(0)).unwrap();
        let res_att = execute_tool(
            &db,
            "get_attachment_info",
            &json!({ "attachment_id": att_id }),
        );
        assert!(!res_att.is_error);
        assert!(res_att.content[0].text.contains("doc.pdf"));

        // Missing param error
        let err_hist = execute_tool(&db, "get_chat_history", &json!({}));
        assert!(err_hist.is_error);

        let err_att = execute_tool(&db, "get_attachment_info", &json!({}));
        assert!(err_att.is_error);

        let not_found_att = execute_tool(
            &db,
            "get_attachment_info",
            &json!({ "attachment_id": "none" }),
        );
        assert!(not_found_att.is_error);

        // Unknown tool
        let unknown = execute_tool(&db, "random_tool", &json!({}));
        assert!(unknown.is_error);
    }
}
