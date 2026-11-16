use serde_json::Value;

/// Maximum number of metadata pairs to index per message
const MAX_METADATA_ENTRIES: usize = 100;
/// Maximum length of a metadata value to store
const MAX_VALUE_LEN: usize = 500;

/// Recursively extract flattened key-value pairs from arbitrary JSON
pub fn extract_metadata(payload: &Value) -> Vec<(String, String)> {
    let mut entries = Vec::new();
    flatten_json(payload, "", &mut entries);
    entries.truncate(MAX_METADATA_ENTRIES);
    entries
}

fn flatten_json(val: &Value, prefix: &str, out: &mut Vec<(String, String)>) {
    if out.len() >= MAX_METADATA_ENTRIES {
        return;
    }

    match val {
        Value::Object(map) => {
            for (k, v) in map {
                // Skip base64 or huge binary blobs in metadata
                if k == "base64" || k == "data" && v.is_string() {
                    continue;
                }
                let new_key = if prefix.is_empty() {
                    k.clone()
                } else {
                    format!("{}.{}", prefix, k)
                };
                flatten_json(v, &new_key, out);
            }
        }
        Value::Array(arr) => {
            for (i, v) in arr.iter().enumerate().take(10) {
                let new_key = format!("{}[{}]", prefix, i);
                flatten_json(v, &new_key, out);
            }
        }
        Value::String(s) => {
            if !prefix.is_empty() && !s.is_empty() && s.len() <= MAX_VALUE_LEN {
                out.push((prefix.to_string(), s.clone()));
            }
        }
        Value::Number(n) => {
            if !prefix.is_empty() {
                out.push((prefix.to_string(), n.to_string()));
            }
        }
        Value::Bool(b) => {
            if !prefix.is_empty() {
                out.push((prefix.to_string(), b.to_string()));
            }
        }
        Value::Null => {}
    }
}

/// Format key-value pairs for FTS5 indexing
pub fn format_metadata_text(entries: &[(String, String)]) -> String {
    entries
        .iter()
        .map(|(k, v)| format!("{}:{}", k, v))
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_metadata_nested() {
        let payload = json!({
            "session": "default",
            "key": {
                "remoteJid": "123@s.whatsapp.net",
                "fromMe": false
            },
            "pushName": "Alice",
            "messageTimestamp": 1710000000,
            "base64": "huge_binary_payload_should_be_skipped"
        });

        let entries = extract_metadata(&payload);
        let text = format_metadata_text(&entries);

        assert!(entries.iter().any(|(k, v)| k == "session" && v == "default"));
        assert!(entries.iter().any(|(k, v)| k == "key.remoteJid" && v == "123@s.whatsapp.net"));
        assert!(entries.iter().any(|(k, v)| k == "pushName" && v == "Alice"));
        assert!(!entries.iter().any(|(k, _)| k == "base64"));
        assert!(text.contains("session:default"));
        assert!(text.contains("pushName:Alice"));
    }
}
