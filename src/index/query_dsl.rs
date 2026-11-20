use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use crate::index::lucene::{parse_flexible_date, parse_lucene_string, val_to_timestamp};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryNode {
    Text(String),
    Bool { bool: BoolQuery },
    Match { r#match: HashMap<String, serde_json::Value> },
    Term { term: HashMap<String, serde_json::Value> },
    Range { range: HashMap<String, RangeClause> },
    QueryString { query_string: QueryStringClause },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BoolQuery {
    #[serde(default)]
    pub must: Vec<QueryNode>,
    #[serde(default)]
    pub should: Vec<QueryNode>,
    #[serde(default)]
    pub filter: Vec<QueryNode>,
    #[serde(default)]
    pub must_not: Vec<QueryNode>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct RangeClause {
    pub gte: Option<serde_json::Value>,
    pub lte: Option<serde_json::Value>,
    pub gt: Option<serde_json::Value>,
    pub lt: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct QueryStringClause {
    pub query: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, ToSchema)]
pub struct SearchRequest {
    #[schema(value_type = Option<Object>)]
    pub query: Option<serde_json::Value>,
    pub query_string: Option<String>,
    pub from_date: Option<String>,
    pub to_date: Option<String>,
    pub source: Option<String>,
    pub account: Option<String>,
    pub chat_id: Option<String>,
    pub sender: Option<String>,
    pub media_type: Option<String>,
    pub has_media: Option<bool>,
    pub is_from_me: Option<bool>,
    pub metadata: Option<HashMap<String, String>>,
    pub sort: Option<Vec<HashMap<String, String>>>,
    pub limit: Option<usize>,
    pub offset: Option<usize>,
}

#[derive(Debug, Clone, Default)]
pub struct CompiledQuery {
    pub text_terms: Vec<String>,
    pub source: Option<String>,
    pub account: Option<String>,
    pub chat_id: Option<String>,
    pub sender: Option<String>,
    pub from_timestamp: Option<i64>,
    pub to_timestamp: Option<i64>,
    pub has_media: Option<bool>,
    pub media_type: Option<String>,
    pub is_from_me: Option<bool>,
    pub metadata_filters: Vec<(String, String)>,
    pub sort_asc: bool,
    pub limit: usize,
    pub offset: usize,
}

impl SearchRequest {
    pub fn compile(&self) -> CompiledQuery {
        let mut out = CompiledQuery {
            source: self.source.clone(),
            account: self.account.clone(),
            chat_id: self.chat_id.clone(),
            sender: self.sender.clone(),
            media_type: self.media_type.clone(),
            has_media: self.has_media,
            is_from_me: self.is_from_me,
            from_timestamp: self.from_date.as_deref().and_then(parse_flexible_date),
            to_timestamp: self.to_date.as_deref().and_then(parse_flexible_date),
            limit: self.limit.unwrap_or(50).clamp(1, 100),
            offset: self.offset.unwrap_or(0),
            ..Default::default()
        };

        if let Some(sort_list) = &self.sort {
            for item in sort_list {
                if let Some(order) = item.get("timestamp") {
                    out.sort_asc = order.eq_ignore_ascii_case("asc");
                }
            }
        }
        if let Some(meta) = &self.metadata {
            for (k, v) in meta {
                out.metadata_filters.push((k.clone(), v.clone()));
            }
        }
        if let Some(qs) = &self.query_string {
            parse_lucene_string(qs, &mut out);
        }
        if let Some(q_val) = &self.query {
            if let Ok(q_node) = serde_json::from_value::<QueryNode>(q_val.clone()) {
                compile_query_node(&q_node, &mut out);
            }
        }
        out
    }
}

fn compile_query_node(node: &QueryNode, out: &mut CompiledQuery) {
    match node {
        QueryNode::Text(s) => parse_lucene_string(s, out),
        QueryNode::QueryString { query_string } => parse_lucene_string(&query_string.query, out),
        QueryNode::Term { term } => {
            for (k, v) in term {
                apply_field_term(k, v, out);
            }
        }
        QueryNode::Match { r#match } => {
            for (k, v) in r#match {
                if k == "body" || k == "text" {
                    if let Some(s) = v.as_str() {
                        out.text_terms.push(s.to_string());
                    }
                } else {
                    apply_field_term(k, v, out);
                }
            }
        }
        QueryNode::Range { range } => {
            for (k, r) in range {
                if k == "timestamp" || k == "date" {
                    if let Some(gte) = &r.gte { out.from_timestamp = val_to_timestamp(gte); }
                    if let Some(lte) = &r.lte { out.to_timestamp = val_to_timestamp(lte); }
                }
            }
        }
        QueryNode::Bool { bool } => {
            for child in &bool.must { compile_query_node(child, out); }
            for child in &bool.filter { compile_query_node(child, out); }
            for child in &bool.should { compile_query_node(child, out); }
        }
    }
}

fn apply_field_term(k: &str, v: &serde_json::Value, out: &mut CompiledQuery) {
    let s = v.as_str().map(|s| s.to_string()).unwrap_or_else(|| v.to_string());
    match k {
        "source" => out.source = Some(s),
        "account" | "acct" => out.account = Some(s),
        "chat_id" | "chat" => out.chat_id = Some(s),
        "sender" | "from" => out.sender = Some(s),
        "media_type" | "type" => out.media_type = Some(s),
        "has_media" | "has_attachments" => out.has_media = v.as_bool().or_else(|| s.parse().ok()),
        "is_from_me" => out.is_from_me = v.as_bool().or_else(|| s.parse().ok()),
        _ => {
            let meta_key = k.strip_prefix("metadata.").unwrap_or(k);
            out.metadata_filters.push((meta_key.to_string(), s));
        }
    }
}
