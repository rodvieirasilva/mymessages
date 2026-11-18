use super::*;

#[test]
fn test_parse_text_webhook() {
    let payload = serde_json::json!({
        "event": "messages.upsert",
        "data": {
            "key": {
                "remoteJid": "5511999998888@s.whatsapp.net",
                "fromMe": false,
                "id": "3EB0012345"
            },
            "pushName": "Alice",
            "message": {
                "conversation": "Hello from Evolution API!"
            },
            "messageTimestamp": 1700000000
        }
    });

    let adapter = EvolutionApiAdapter::new();
    let msg = adapter.parse_webhook(&payload).unwrap().unwrap();

    assert_eq!(msg.id, "3EB0012345");
    assert_eq!(msg.source, "whatsapp");
    assert_eq!(msg.conversation_id, "5511999998888@s.whatsapp.net");
    assert_eq!(msg.sender.name.as_deref(), Some("Alice"));
    assert_eq!(msg.body.as_deref(), Some("Hello from Evolution API!"));
    assert!(!msg.is_from_me);
}

#[test]
fn test_parse_image_webhook_with_base64() {
    let b64 = BASE64.encode(b"fake jpeg data");
    let payload = serde_json::json!({
        "event": "messages.upsert",
        "data": {
            "key": {
                "remoteJid": "120363024@g.us",
                "fromMe": true,
                "id": "3EB0099999"
            },
            "pushName": "Me",
            "message": {
                "imageMessage": {
                    "caption": "Photo in group",
                    "mimetype": "image/jpeg",
                    "base64": b64
                }
            },
            "messageTimestamp": 1700000050
        }
    });

    let adapter = EvolutionApiAdapter::new();
    let msg = adapter.parse_webhook(&payload).unwrap().unwrap();

    assert_eq!(msg.id, "3EB0099999");
    assert!(msg.is_group);
    assert!(msg.is_from_me);
    assert_eq!(msg.body.as_deref(), Some("Photo in group"));
    assert_eq!(msg.attachments.len(), 1);
    assert_eq!(msg.attachments[0].mimetype, "image/jpeg");
    assert_eq!(msg.attachments[0].data, b"fake jpeg data");
}
