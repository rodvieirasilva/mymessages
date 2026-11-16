use chrono::{DateTime, NaiveDate};
use crate::index::query_dsl::CompiledQuery;

pub fn parse_lucene_string(input: &str, out: &mut CompiledQuery) {
    for part in input.split_whitespace() {
        if let Some((k, v)) = part.split_once(':') {
            let k = k.to_lowercase();
            let v = v.trim_matches('"');
            match k.as_str() {
                "from" | "sender" => out.sender = Some(v.to_string()),
                "chat" | "to" => out.chat_id = Some(v.to_string()),
                "source" => out.source = Some(v.to_string()),
                "type" | "media" => out.media_type = Some(v.to_string()),
                "has" => {
                    if v == "media" || v == "attachment" || v == "file" {
                        out.has_media = Some(true);
                    }
                }
                "is" => {
                    if v == "incoming" {
                        out.is_from_me = Some(false);
                    } else if v == "outgoing" || v == "from_me" {
                        out.is_from_me = Some(true);
                    }
                }
                "after" | "since" => out.from_timestamp = parse_flexible_date(v),
                "before" | "until" => out.to_timestamp = parse_flexible_date(v),
                other => {
                    if let Some(meta_key) = other.strip_prefix("meta.") {
                        out.metadata_filters.push((meta_key.to_string(), v.to_string()));
                    } else {
                        out.text_terms.push(part.to_string());
                    }
                }
            }
        } else {
            out.text_terms.push(part.to_string());
        }
    }
}

pub fn parse_flexible_date(input: &str) -> Option<i64> {
    if let Ok(ts) = input.parse::<i64>() {
        return Some(ts);
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(input) {
        return Some(dt.timestamp());
    }
    if let Ok(d) = NaiveDate::parse_from_str(input, "%Y-%m-%d") {
        return Some(d.and_hms_opt(0, 0, 0)?.and_utc().timestamp());
    }
    None
}

pub fn val_to_timestamp(v: &serde_json::Value) -> Option<i64> {
    if let Some(n) = v.as_i64() {
        Some(n)
    } else if let Some(s) = v.as_str() {
        parse_flexible_date(s)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_lucene_string_full() {
        let mut q = CompiledQuery::default();
        parse_lucene_string("contract from:Alice after:2024-01-01 type:document is:incoming meta.session:default", &mut q);

        assert_eq!(q.text_terms, vec!["contract"]);
        assert_eq!(q.sender, Some("Alice".to_string()));
        assert_eq!(q.media_type, Some("document".to_string()));
        assert_eq!(q.is_from_me, Some(false));
        assert!(q.from_timestamp.is_some());
        assert_eq!(q.metadata_filters, vec![("session".to_string(), "default".to_string())]);
    }

    #[test]
    fn test_parse_dates() {
        assert!(parse_flexible_date("2024-01-01").is_some());
        assert!(parse_flexible_date("1710000000").is_some());
        assert!(parse_flexible_date("invalid").is_none());
    }
}
