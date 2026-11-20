use std::path::PathBuf;
use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;
use chrono::{DateTime, Utc};
use crate::models::{CanonicalAttachment, CanonicalMessage, CanonicalParticipant};

pub fn extract_header(eml: &str, header: &str) -> Option<String> {
    let target = format!("{}: ", header);
    for line in eml.lines() {
        if line.starts_with(&target) {
            return Some(line[target.len()..].trim().to_string());
        }
    }
    None
}

pub fn parse_eml_to_canonical(eml: &str) -> Option<CanonicalMessage> {
    let source = extract_header(eml, "X-MyMessages-Source").unwrap_or_else(|| "unknown".into());
    let account = extract_header(eml, "X-MyMessages-Account").unwrap_or_else(|| "default".into());
    let conv_id = extract_header(eml, "X-MyMessages-Chat-ID").unwrap_or_else(|| "chat".into());
    let msg_id = extract_header(eml, "X-MyMessages-Message-ID")
        .or_else(|| extract_header(eml, "Message-ID"))
        .unwrap_or_else(|| "msg".into());

    let subject = extract_header(eml, "Subject");
    let is_from_me = extract_header(eml, "X-MyMessages-Direction").as_deref() == Some("outgoing");
    let is_group = extract_header(eml, "X-MyMessages-Is-Group")
        .and_then(|v| v.parse().ok())
        .unwrap_or_else(|| conv_id.ends_with("@g.us"));

    let ts_str = extract_header(eml, "X-MyMessages-Timestamp");
    let timestamp = ts_str
        .and_then(|s| DateTime::parse_from_rfc3339(&s).ok().map(|d| d.with_timezone(&Utc)))
        .unwrap_or_else(Utc::now);

    let sender_raw = extract_header(eml, "From").unwrap_or_default();
    let sender_name = extract_header(eml, "X-MyMessages-Sender-Name").or_else(|| {
        sender_raw.find('"').and_then(|start| {
            sender_raw[start + 1..].find('"').map(|end| sender_raw[start + 1..start + 1 + end].to_string())
        })
    });
    let sender_id = extract_header(eml, "X-MyMessages-Sender-ID").unwrap_or_else(|| {
        if is_from_me { "me".into() } else { "sender".into() }
    });
    let sender_phone = extract_header(eml, "X-MyMessages-Sender-Phone");
    let reply_to_id = extract_header(eml, "X-MyMessages-Reply-To");

    let (body, attachments) = extract_eml_body_and_attachments(eml);

    Some(CanonicalMessage {
        id: msg_id,
        source,
        account,
        conversation_id: conv_id,
        conversation_name: subject,
        is_group,
        sender: CanonicalParticipant {
            id: sender_id,
            name: sender_name,
            phone_number: sender_phone,
        },
        recipients: Vec::new(),
        timestamp,
        body,
        attachments,
        reply_to_id,
        is_from_me,
        raw_payload: serde_json::Value::Null,
    })
}

fn extract_eml_body_and_attachments(eml: &str) -> (Option<String>, Vec<CanonicalAttachment>) {
    let sep = if eml.contains("\r\n\r\n") { "\r\n\r\n" } else { "\n\n" };
    let parts: Vec<&str> = eml.splitn(2, sep).collect();
    if parts.len() < 2 {
        return (None, Vec::new());
    }

    let body_section = parts[1];
    if !eml.contains("multipart/mixed") {
        let trimmed = body_section.trim();
        return (if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }, Vec::new());
    }

    let boundary = match eml.lines().find_map(|line| {
        if line.to_lowercase().contains("boundary=\"") {
            line.find("boundary=\"").and_then(|s| {
                line[s + 10..].find('"').map(|e| line[s + 10..s + 10 + e].to_string())
            })
        } else {
            None
        }
    }) {
        Some(b) => b,
        None => return (None, Vec::new()),
    };

    let mut body = None;
    let mut attachments = Vec::new();
    let delimiter = format!("--{}", boundary);

    for part in body_section.split(&delimiter) {
        let part_trimmed = part.trim();
        if part_trimmed.is_empty() || part_trimmed == "--" {
            continue;
        }
        if part_trimmed.contains("Content-Type: text/plain") && !part_trimmed.contains("Content-Disposition: attachment") {
            if let Some(pos) = part_trimmed.find("\r\n\r\n").or_else(|| part_trimmed.find("\n\n")) {
                let text = part_trimmed[pos..].trim();
                if !text.is_empty() {
                    body = Some(text.to_string());
                }
            }
        } else if part_trimmed.contains("X-Attachment-SHA256:") || part_trimmed.contains("Content-Disposition: attachment") {
            if let Some(att) = parse_attachment_part(part_trimmed) {
                attachments.push(att);
            }
        }
    }

    (body, attachments)
}

fn parse_attachment_part(part_trimmed: &str) -> Option<CanonicalAttachment> {
    let filename = part_trimmed.lines().find_map(|l| {
        let lower = l.to_lowercase();
        if lower.starts_with("content-disposition:") || lower.starts_with("content-type:") {
            l.find("filename=\"").and_then(|s| {
                l[s + 10..].find('"').map(|e| l[s + 10..s + 10 + e].to_string())
            })
        } else {
            None
        }
    });

    let mimetype = part_trimmed.lines().find_map(|l| {
        if l.to_lowercase().starts_with("content-type:") {
            let val = l["content-type:".len()..].trim();
            let mime = val.split(';').next().unwrap_or("application/octet-stream").trim();
            Some(mime.to_string())
        } else {
            None
        }
    }).unwrap_or_else(|| "application/octet-stream".into());

    if let Some(ext_path) = extract_header(part_trimmed, "X-Attachment-External-Path") {
        let sha256 = extract_header(part_trimmed, "X-Attachment-SHA256").unwrap_or_default();
        let file_size = extract_header(part_trimmed, "X-Attachment-Size")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);
        return Some(CanonicalAttachment::from_external(
            filename,
            mimetype,
            PathBuf::from(ext_path),
            file_size,
            sha256,
        ));
    }

    let sep_pos = part_trimmed.find("\r\n\r\n").or_else(|| part_trimmed.find("\n\n"))?;
    let raw_b64: String = part_trimmed[sep_pos..]
        .chars()
        .filter(|c| !c.is_whitespace())
        .collect();
    let data = BASE64.decode(raw_b64).ok()?;
    Some(CanonicalAttachment::new(filename, mimetype, data))
}
