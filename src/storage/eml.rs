use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::SecondsFormat;

use crate::models::CanonicalMessage;

pub fn build_eml(msg: &CanonicalMessage, content_hash: &str) -> String {
    let boundary = format!("==_mymessages_{}_==", msg.id.replace(['<', '>', '@', ' '], "_"));
    let mut out = String::new();

    let sender_name = msg.sender.name.as_deref().unwrap_or("Unknown");
    let sender_addr = format!("{}@{}.mymessages.local", msg.sender.id, msg.source);
    let subject = msg.conversation_name.as_deref().unwrap_or("Chat message");

    out.push_str(&format!("Message-ID: <{}@{}.mymessages.local>\r\n", msg.id, msg.source));
    out.push_str(&format!("Date: {}\r\n", msg.timestamp.to_rfc2822()));
    out.push_str(&format!("From: \"{}\" <{}>\r\n", sender_name, sender_addr));
    out.push_str(&format!("Subject: {}\r\n", subject));
    out.push_str("MIME-Version: 1.0\r\n");
    out.push_str(&format!("X-MyMessages-Source: {}\r\n", msg.source));
    out.push_str(&format!("X-MyMessages-Chat-ID: {}\r\n", msg.conversation_id));
    out.push_str(&format!("X-MyMessages-Message-ID: {}\r\n", msg.id));
    out.push_str(&format!("X-MyMessages-Sender-ID: {}\r\n", msg.sender.id));
    if let Some(ref sname) = msg.sender.name {
        let clean_sname = sname.replace(['\r', '\n'], " ");
        out.push_str(&format!("X-MyMessages-Sender-Name: {}\r\n", clean_sname));
    }
    if let Some(ref phone) = msg.sender.phone_number {
        out.push_str(&format!("X-MyMessages-Sender-Phone: {}\r\n", phone));
    }
    out.push_str(&format!("X-MyMessages-Is-Group: {}\r\n", msg.is_group));
    if let Some(ref reply_to) = msg.reply_to_id {
        out.push_str(&format!("X-MyMessages-Reply-To: {}\r\n", reply_to));
    }
    out.push_str(&format!("X-MyMessages-SHA256: {}\r\n", content_hash));
    out.push_str(&format!(
        "X-MyMessages-Direction: {}\r\n",
        if msg.is_from_me { "outgoing" } else { "incoming" }
    ));
    out.push_str(&format!(
        "X-MyMessages-Timestamp: {}\r\n",
        msg.timestamp.to_rfc3339_opts(SecondsFormat::Secs, true)
    ));

    if msg.attachments.is_empty() {
        out.push_str("Content-Type: text/plain; charset=\"utf-8\"\r\n");
        out.push_str("Content-Transfer-Encoding: 8bit\r\n\r\n");
        out.push_str(msg.body.as_deref().unwrap_or(""));
        out.push_str("\r\n");
    } else {
        out.push_str(&format!("Content-Type: multipart/mixed; boundary=\"{}\"\r\n\r\n", boundary));

        // Text body part
        out.push_str(&format!("--{}\r\n", boundary));
        out.push_str("Content-Type: text/plain; charset=\"utf-8\"\r\n");
        out.push_str("Content-Transfer-Encoding: 8bit\r\n\r\n");
        out.push_str(msg.body.as_deref().unwrap_or(""));
        out.push_str("\r\n\r\n");

        // Attachment parts
        for att in &msg.attachments {
            let fname = att.filename.as_deref().unwrap_or("attachment.bin");
            let b64_data = BASE64.encode(&att.data);

            out.push_str(&format!("--{}\r\n", boundary));
            out.push_str(&format!("Content-Type: {}; name=\"{}\"\r\n", att.mimetype, fname));
            out.push_str(&format!("Content-Disposition: attachment; filename=\"{}\"\r\n", fname));
            out.push_str("Content-Transfer-Encoding: base64\r\n");
            out.push_str(&format!("X-Attachment-SHA256: {}\r\n\r\n", att.sha256));

            // Chunk base64 lines for RFC compliance
            for chunk in b64_data.as_bytes().chunks(76) {
                out.push_str(std::str::from_utf8(chunk).unwrap_or(""));
                out.push_str("\r\n");
            }
            out.push_str("\r\n");
        }

        out.push_str(&format!("--{}--\r\n", boundary));
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{CanonicalAttachment, CanonicalParticipant};
    use chrono::Utc;

    #[test]
    fn test_build_eml_text_only() {
        let msg = CanonicalMessage {
            id: "msg_1".into(),
            source: "whatsapp".into(),
            conversation_id: "conv_1".into(),
            conversation_name: Some("My Group".into()),
            is_group: true,
            sender: CanonicalParticipant {
                id: "user_1".into(),
                name: Some("Alice".into()),
                phone_number: None,
            },
            recipients: vec![],
            timestamp: Utc::now(),
            body: Some("Simple text".into()),
            attachments: vec![],
            reply_to_id: None,
            is_from_me: false,
            raw_payload: serde_json::json!({}),
        };

        let eml = build_eml(&msg, "hash123");
        assert!(eml.contains("Message-ID: <msg_1@whatsapp.mymessages.local>"));
        assert!(eml.contains("X-MyMessages-SHA256: hash123"));
        assert!(eml.contains("Simple text"));
    }

    #[test]
    fn test_build_eml_with_attachments() {
        let att = CanonicalAttachment::new(Some("test.jpg".into()), "image/jpeg".into(), b"imagebytes".to_vec());
        let msg = CanonicalMessage {
            id: "msg_2".into(),
            source: "whatsapp".into(),
            conversation_id: "conv_2".into(),
            conversation_name: None,
            is_group: false,
            sender: CanonicalParticipant {
                id: "user_2".into(),
                name: None,
                phone_number: None,
            },
            recipients: vec![],
            timestamp: Utc::now(),
            body: Some("Photo caption".into()),
            attachments: vec![att],
            reply_to_id: None,
            is_from_me: true,
            raw_payload: serde_json::json!({}),
        };

        let eml = build_eml(&msg, "hash456");
        assert!(eml.contains("multipart/mixed"));
        assert!(eml.contains("name=\"test.jpg\""));
        assert!(eml.contains("X-MyMessages-Direction: outgoing"));
    }
}
