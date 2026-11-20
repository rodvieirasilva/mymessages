use serde_json::{json, Value};
use crate::index::{get_chat_messages, list_accounts, list_chats, search_compiled, Database, SearchRequest};
use super::protocol::{McpCallToolResult, McpTool};

pub fn get_available_tools() -> Vec<McpTool> {
    vec![
        McpTool {
            name: "search_messages".into(),
            description: "Search messages across accounts in the vault using SQLite FTS5, Lucene query syntax, or Elasticsearch JSON Query DSL.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search keyword or Lucene query (e.g. 'contract from:Alice account:phone_a after:2024-01-01')" },
                    "query_dsl": { "type": "object", "description": "Optional Elasticsearch-style JSON Query DSL object" },
                    "from_date": { "type": "string", "description": "Filter on or after this date (ISO-8601 or YYYY-MM-DD)" },
                    "to_date": { "type": "string", "description": "Filter on or before this date (ISO-8601 or YYYY-MM-DD)" },
                    "sender": { "type": "string", "description": "Filter by sender name or ID" },
                    "chat_id": { "type": "string", "description": "Optional conversation ID or remote JID" },
                    "source": { "type": "string", "description": "Source filter: 'whatsapp', 'whatsapp_business', 'folder', etc." },
                    "account": { "type": "string", "description": "Optional account ID filter (omit to search across all accounts)" },
                    "has_media": { "type": "boolean", "description": "Filter messages with attachments" },
                    "media_type": { "type": "string", "description": "Filter by media type: 'image', 'audio', 'document', 'video'" },
                    "is_from_me": { "type": "boolean", "description": "true for outgoing, false for incoming" },
                    "metadata": { "type": "object", "description": "Key-value map of arbitrary metadata filters" },
                    "limit": { "type": "integer", "description": "Max results to return (default 20, max 100)" }
                }
            }),
        },
        McpTool {
            name: "list_accounts".into(),
            description: "List all indexed messaging accounts in the vault with message and conversation counts.".into(),
            input_schema: json!({ "type": "object", "properties": {} }),
        },
        McpTool {
            name: "list_chats".into(),
            description: "List conversations and group chats in the vault with metadata including account, participant names, message counts, and last activity.".into(),
            input_schema: json!({
                "type": "object",
                "properties": {
                    "source": { "type": "string", "description": "Optional source filter (e.g. 'whatsapp')" },
                    "account": { "type": "string", "description": "Optional account filter" },
                    "query": { "type": "string", "description": "Optional query filter matching contact name, ID, or message text" }
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
            description: "Retrieve metadata, MIME type, SHA-256 hash, and storage path for an attachment.".into(),
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
            let limit = args.get("limit").and_then(|l| l.as_u64()).map(|u| u as usize).unwrap_or(20);
            let mut req = if let Some(dsl) = args.get("query_dsl") {
                SearchRequest { query: Some(dsl.clone()), ..Default::default() }
            } else {
                let query_str = args.get("query").and_then(|q| q.as_str()).map(|s| s.to_string());
                SearchRequest { query_string: query_str, ..Default::default() }
            };

            req.from_date = args.get("from_date").and_then(|d| d.as_str()).map(|s| s.to_string());
            req.to_date = args.get("to_date").and_then(|d| d.as_str()).map(|s| s.to_string());
            req.sender = args.get("sender").and_then(|s| s.as_str()).map(|s| s.to_string());
            req.chat_id = args.get("chat_id").and_then(|c| c.as_str()).map(|s| s.to_string());
            req.source = args.get("source").and_then(|s| s.as_str()).map(|s| s.to_string());
            req.account = args.get("account").and_then(|a| a.as_str()).map(|s| s.to_string());
            req.has_media = args.get("has_media").and_then(|h| h.as_bool());
            req.media_type = args.get("media_type").and_then(|m| m.as_str()).map(|s| s.to_string());
            req.is_from_me = args.get("is_from_me").and_then(|f| f.as_bool());
            req.limit = Some(limit);

            if let Some(meta_obj) = args.get("metadata").and_then(|m| m.as_object()) {
                let mut map = std::collections::HashMap::new();
                for (k, v) in meta_obj {
                    let val_str = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
                    map.insert(k.clone(), val_str);
                }
                req.metadata = Some(map);
            }

            let compiled = req.compile();
            match search_compiled(db.conn(), &compiled) {
                Ok(results) => {
                    let json_text = serde_json::to_string_pretty(&results).unwrap_or_default();
                    McpCallToolResult::text(format!("Found {} messages:\n{}", results.len(), json_text))
                }
                Err(e) => McpCallToolResult::error(format!("Search failed: {}", e)),
            }
        }

        "list_accounts" => match list_accounts(db.conn()) {
            Ok(accounts) => {
                let json_text = serde_json::to_string_pretty(&accounts).unwrap_or_default();
                McpCallToolResult::text(format!("Retrieved {} accounts:\n{}", accounts.len(), json_text))
            }
            Err(e) => McpCallToolResult::error(format!("Failed to list accounts: {}", e)),
        },

        "list_chats" => {
            let source = args.get("source").and_then(|s| s.as_str());
            let account = args.get("account").and_then(|a| a.as_str());
            let query = args.get("query").and_then(|q| q.as_str());
            match list_chats(db.conn(), source, account, query) {
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
                SELECT a.id, a.message_id, a.mimetype, a.filename, a.file_size, a.sha256, a.media_type, m.eml_path, a.external_path
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
                    "external_path": row.get::<_, Option<String>>(8)?,
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
#[path = "tools_test.rs"]
mod tests;
