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
        raw_payload: json!({ "session": "work_vault" }),
    };
    index_message(db.conn_mut(), &msg, "hash_mcp_1", "mail/wh/cur/1.eml").unwrap();
    db
}

#[test]
fn test_execute_mcp_tools() {
    let db = setup_test_db();

    // 1. Basic search_messages
    let res = execute_tool(&db, "search_messages", &json!({ "query": "contract" }));
    assert!(!res.is_error);
    assert!(res.content[0].text.contains("Found 1 messages"));

    // 2. Lucene query syntax via query string in MCP
    let res_lucene = execute_tool(
        &db,
        "search_messages",
        &json!({ "query": "contract from:Dave type:document" }),
    );
    assert!(!res_lucene.is_error);
    assert!(res_lucene.content[0].text.contains("Found 1 messages"));

    // 3. Elasticsearch-style JSON Query DSL in MCP
    let res_dsl = execute_tool(
        &db,
        "search_messages",
        &json!({
            "query_dsl": {
                "bool": {
                    "must": [{ "match": { "body": "contract" } }],
                    "filter": [{ "term": { "media_type": "document" } }]
                }
            }
        }),
    );
    assert!(!res_dsl.is_error);
    assert!(res_dsl.content[0].text.contains("Found 1 messages"));

    // 4. list_chats
    let res_chats = execute_tool(&db, "list_chats", &json!({}));
    assert!(!res_chats.is_error);
    assert!(res_chats.content[0].text.contains("Retrieved 1 conversations"));

    let res_chats_filter = execute_tool(&db, "list_chats", &json!({ "query": "Project" }));
    assert!(!res_chats_filter.is_error);
    assert!(res_chats_filter.content[0].text.contains("Retrieved 1 conversations"));

    let res_chats_none = execute_tool(&db, "list_chats", &json!({ "query": "Nonexistent" }));
    assert!(!res_chats_none.is_error);
    assert!(res_chats_none.content[0].text.contains("Retrieved 0 conversations"));

    // 5. get_chat_history
    let res_hist = execute_tool(
        &db,
        "get_chat_history",
        &json!({ "chat_id": "whatsapp:chat_mcp_1" }),
    );
    assert!(!res_hist.is_error);
    assert!(res_hist.content[0].text.contains("Retrieved 1 messages"));

    // 6. get_attachment_info
    let att_id: String = db.conn().query_row("SELECT id FROM attachments LIMIT 1", [], |r| r.get(0)).unwrap();
    let res_att = execute_tool(
        &db,
        "get_attachment_info",
        &json!({ "attachment_id": att_id }),
    );
    assert!(!res_att.is_error);
    assert!(res_att.content[0].text.contains("doc.pdf"));

    // Error handling
    assert!(execute_tool(&db, "get_chat_history", &json!({})).is_error);
    assert!(execute_tool(&db, "get_attachment_info", &json!({})).is_error);
    assert!(execute_tool(&db, "get_attachment_info", &json!({ "attachment_id": "none" })).is_error);
    assert!(execute_tool(&db, "random_tool", &json!({})).is_error);
}
