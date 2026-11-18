use super::*;
use crate::index::{index_message, Database};
use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};
use chrono::Utc;

#[test]
fn test_search_and_metadata_filter() {
    let mut db = Database::in_memory().unwrap();

    let msg = CanonicalMessage {
        id: "msg_fts_1".into(),
        source: "whatsapp".into(),
        conversation_id: "conv_fts".into(),
        conversation_name: Some("Test Group".into()),
        is_group: true,
        sender: CanonicalParticipant {
            id: "user_fts".into(),
            name: Some("Alice".into()),
            phone_number: Some("+123456789".into()),
        },
        recipients: Vec::new(),
        timestamp: Utc::now(),
        body: Some("Contract proposal for the new messaging vault".into()),
        attachments: vec![CanonicalAttachment::new(
            Some("contract.pdf".into()),
            "application/pdf".into(),
            b"pdf-content".to_vec(),
        )],
        reply_to_id: None,
        is_from_me: false,
        raw_payload: serde_json::json!({
            "order": {
                "id": "ORD-12345",
                "customer": "ACME Corp"
            }
        }),
    };

    index_message(db.conn_mut(), &msg, "hash_fts_1", "mail/wh/cur/1.eml").unwrap();

    // 1. Text search
    let q = SearchQuery {
        query: "Contract proposal".into(),
        ..Default::default()
    };
    let results = search_messages(db.conn(), &q).unwrap();
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "whatsapp:msg_fts_1");
    assert!(results[0].snippet.is_some());
    assert_eq!(results[0].attachments.len(), 1);

    // 2. Metadata search
    let q_meta = SearchQuery {
        query: "meta.order.id:ORD-12345".into(),
        ..Default::default()
    };
    let res_meta = search_messages(db.conn(), &q_meta).unwrap();
    assert_eq!(res_meta.len(), 1);
    assert_eq!(res_meta[0].id, "whatsapp:msg_fts_1");

    // 3. Multi-word unquoted search
    let q_multi = SearchQuery {
        query: "proposal vault".into(),
        ..Default::default()
    };
    let res_multi = search_messages(db.conn(), &q_multi).unwrap();
    assert_eq!(res_multi.len(), 1);
}
