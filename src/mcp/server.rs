use anyhow::Result;
use serde_json::json;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::index::Database;
use super::protocol::{JsonRpcRequest, JsonRpcResponse};
use super::tools::{execute_tool, get_available_tools};

pub fn handle_request(db: &Database, req: &JsonRpcRequest) -> Option<JsonRpcResponse> {
    match req.method.as_str() {
        "initialize" => {
            let res = json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {
                    "tools": {}
                },
                "serverInfo": {
                    "name": "mymessages",
                    "version": env!("CARGO_PKG_VERSION")
                }
            });
            Some(JsonRpcResponse::success(req.id.clone(), res))
        }

        "notifications/initialized" => None,

        "ping" => Some(JsonRpcResponse::success(req.id.clone(), json!({}))),

        "tools/list" => {
            let tools = get_available_tools();
            Some(JsonRpcResponse::success(req.id.clone(), json!({ "tools": tools })))
        }

        "tools/call" => {
            let name = req.params.get("name").and_then(|n| n.as_str()).unwrap_or("");
            let args = req.params.get("arguments").unwrap_or(&serde_json::Value::Null);
            let result = execute_tool(db, name, args);
            Some(JsonRpcResponse::success(req.id.clone(), serde_json::to_value(result).unwrap_or_default()))
        }

        _ => Some(JsonRpcResponse::error(
            req.id.clone(),
            -32601,
            format!("Method not found: {}", req.method),
        )),
    }
}

pub async fn run_mcp_server(db: Database) -> Result<()> {
    let stdin = tokio::io::stdin();
    let mut stdout = tokio::io::stdout();
    let mut reader = BufReader::new(stdin).lines();

    while let Ok(Some(line)) = reader.next_line().await {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(trimmed) {
            Ok(r) => r,
            Err(e) => {
                let err_resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {}", e));
                let mut out = serde_json::to_string(&err_resp)?;
                out.push('\n');
                stdout.write_all(out.as_bytes()).await?;
                stdout.flush().await?;
                continue;
            }
        };

        if let Some(resp) = handle_request(&db, &req) {
            let mut out = serde_json::to_string(&resp)?;
            out.push('\n');
            stdout.write_all(out.as_bytes()).await?;
            stdout.flush().await?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mcp_initialize_and_tools_list() {
        let db = Database::in_memory().unwrap();

        // 1. Initialize
        let init_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(1)),
            method: "initialize".into(),
            params: json!({}),
        };
        let resp = handle_request(&db, &init_req).unwrap();
        assert_eq!(resp.id, Some(json!(1)));
        assert_eq!(resp.result.unwrap()["serverInfo"]["name"], "mymessages");

        // 2. Tools list
        let tools_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(2)),
            method: "tools/list".into(),
            params: json!({}),
        };
        let resp = handle_request(&db, &tools_req).unwrap();
        let tools = resp.result.unwrap()["tools"].as_array().unwrap().clone();
        assert_eq!(tools.len(), 5);
        assert_eq!(tools[0]["name"], "search_messages");

        // 3. Call tool search_messages
        let call_req = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(json!(3)),
            method: "tools/call".into(),
            params: json!({
                "name": "search_messages",
                "arguments": { "query": "hello" }
            }),
        };
        let resp = handle_request(&db, &call_req).unwrap();
        assert!(resp.result.is_some());
    }
}
