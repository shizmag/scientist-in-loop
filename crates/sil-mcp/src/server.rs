//! MCP JSON-RPC Server over async stdin/stdout streams.

use tokio::io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader};
use serde_json::json;

use crate::protocol::{JsonRpcRequest, JsonRpcResponse, Tool};
use crate::tools::{call_tool, list_tools};

/// MCP Server instance.
pub struct McpServer {
    tools: Vec<Tool>,
}

impl Default for McpServer {
    fn default() -> Self {
        Self::new()
    }
}

impl McpServer {
    /// Create a new McpServer with all registered tools.
    pub fn new() -> Self {
        Self {
            tools: list_tools(),
        }
    }

    /// Run the MCP server reading JSON-RPC lines from `reader` and writing responses to `writer`.
    pub async fn run<R, W>(&self, reader: R, mut writer: W) -> anyhow::Result<()>
    where
        R: AsyncRead + Unpin,
        W: AsyncWrite + Unpin,
    {
        let mut buf_reader = BufReader::new(reader);
        let mut line = String::new();

        while buf_reader.read_line(&mut line).await? > 0 {
            let trimmed = line.trim();
            if let Some(resp) = if trimmed.is_empty() { None } else { self.handle_request_line(trimmed).await } {
                let mut json_bytes = serde_json::to_vec(&resp)?;
                json_bytes.push(b'\n');
                writer.write_all(&json_bytes).await?;
                writer.flush().await?;
            }
            line.clear();
        }

        Ok(())
    }

    /// Process a single JSON-RPC request line string and generate a response if appropriate.
    pub async fn handle_request_line(&self, line: &str) -> Option<JsonRpcResponse> {
        let req: JsonRpcRequest = match serde_json::from_str(line) {
            Ok(r) => r,
            Err(e) => {
                return Some(JsonRpcResponse::error(
                    None,
                    -32700,
                    format!("Parse error: {e}"),
                ));
            }
        };

        self.handle_request(req).await
    }

    /// Handle a parsed JSON-RPC request.
    pub async fn handle_request(&self, req: JsonRpcRequest) -> Option<JsonRpcResponse> {
        let id = req.id.clone();

        match req.method.as_str() {
            "initialize" => {
                let result = json!({
                    "protocolVersion": "2024-11-05",
                    "capabilities": {
                        "tools": {}
                    },
                    "serverInfo": {
                        "name": "sil-mcp",
                        "version": env!("CARGO_PKG_VERSION")
                    }
                });
                Some(JsonRpcResponse::success(id, result))
            }
            "notifications/initialized" => None,
            "ping" => Some(JsonRpcResponse::success(id, json!({}))),
            "tools/list" => {
                let result = json!({
                    "tools": self.tools
                });
                Some(JsonRpcResponse::success(id, result))
            }
            "tools/call" => {
                let params = req.params.unwrap_or_else(|| json!({}));
                let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
                let arguments = params.get("arguments").cloned();

                let tool_result = call_tool(tool_name, arguments);
                let result_json = serde_json::to_value(&tool_result).unwrap_or_default();
                Some(JsonRpcResponse::success(id, result_json))
            }
            _ => {
                if id.is_some() {
                    Some(JsonRpcResponse::error(
                        id,
                        -32601,
                        format!("Method not found: {}", req.method),
                    ))
                } else {
                    None
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::{CallToolResult, Content};

    #[tokio::test]
    async fn test_jsonrpc_initialize() {
        let server = McpServer::new();
        let req_json = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let response = server.handle_request_line(req_json).await.expect("Expected response for initialize");

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(1)));
        assert!(response.error.is_none());

        let result = response.result.expect("Expected result");
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert_eq!(result["serverInfo"]["name"], "sil-mcp");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[tokio::test]
    async fn test_tools_list() {
        let server = McpServer::new();
        let req_json = r#"{"jsonrpc":"2.0","id":2,"method":"tools/list"}"#;
        let response = server.handle_request_line(req_json).await.expect("Expected response for tools/list");

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, Some(json!(2)));
        assert!(response.error.is_none());

        let result = response.result.expect("Expected result");
        let tools = result["tools"].as_array().expect("tools should be an array");

        assert_eq!(tools.len(), 12, "Should register all 12 core sil MCP tools");

        let expected_names = [
            "sil_search_sources",
            "sil_get_source_context",
            "sil_suggest_citations",
            "sil_list_todos",
            "sil_update_todo",
            "sil_list_skills",
            "sil_invoke_skill",
            "sil_get_workspace_context",
            "sil_get_structure",
            "sil_build_and_doctor",
            "sil_propose_commit",
            "sil_fetch_source",
        ];

        for expected in expected_names {
            let found = tools.iter().any(|t| t["name"] == expected);
            assert!(found, "Tool '{expected}' should be registered");
        }

        for tool in tools {
            assert!(tool["description"].is_string());
            let schema = &tool["inputSchema"];
            assert_eq!(schema["type"], "object");
            assert!(schema["properties"].is_object());
        }
    }

    #[tokio::test]
    async fn test_tool_call_dispatch() {
        let server = McpServer::new();

        // 1. Test sil_suggest_citations
        let req_json = r#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"sil_suggest_citations","arguments":{"query":"generative AI"}}}"#;
        let response = server.handle_request_line(req_json).await.expect("Expected response for tools/call");

        assert_eq!(response.id, Some(json!(3)));
        let result = response.result.expect("Expected result");
        let call_res: CallToolResult = serde_json::from_value(result).expect("Should parse as CallToolResult");

        assert_eq!(call_res.content.len(), 1);
        let Content::Text { text } = &call_res.content[0];
        assert!(text.contains("bibtex"), "Response text should contain bibtex suggestion");

        // 2. Test sil_propose_commit
        let req_json2 = r#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"sil_propose_commit","arguments":{"action":"drafting"}}}"#;
        let response2 = server.handle_request_line(req_json2).await.expect("Expected response for tools/call propose commit");
        assert_eq!(response2.id, Some(json!(4)));

        // 3. Test unknown tool
        let req_json3 = r#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"non_existent_tool"}}"#;
        let response3 = server.handle_request_line(req_json3).await.expect("Expected response for unknown tool");
        let result3 = response3.result.expect("Expected result");
        let call_res3: CallToolResult = serde_json::from_value(result3).expect("Should parse as CallToolResult");
        assert_eq!(call_res3.is_error, Some(true));

        // 4. Test async stream run with tokio duplex
        let (client, server_stream) = tokio::io::duplex(1024);
        let (client_read, mut client_write) = tokio::io::split(client);

        let server_handle = tokio::spawn(async move {
            let (server_read, server_write) = tokio::io::split(server_stream);
            let server_inst = McpServer::new();
            server_inst.run(server_read, server_write).await.unwrap();
        });

        client_write.write_all(b"{\"jsonrpc\":\"2.0\",\"id\":10,\"method\":\"ping\"}\n").await.unwrap();

        let mut reader = BufReader::new(client_read);
        let mut resp_line = String::new();
        reader.read_line(&mut resp_line).await.unwrap();

        assert!(resp_line.contains("\"id\":10"));
        drop(client_write);
        drop(reader);
        let _ = tokio::time::timeout(std::time::Duration::from_millis(200), server_handle).await;
    }

    #[tokio::test]
    async fn test_jsonrpc_parse_error() {
        let server = McpServer::new();
        let invalid_json = r#"{"jsonrpc":"2.0", "method": invalid"#;
        let response = server
            .handle_request_line(invalid_json)
            .await
            .expect("Expected parse error response");

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, None);
        let err = response.error.expect("Expected error object");
        assert_eq!(err.code, -32700);
        assert!(err.message.starts_with("Parse error:"));
    }

    #[tokio::test]
    async fn test_jsonrpc_unknown_method_and_notifications() {
        let server = McpServer::new();

        // Unknown method with id -> returns error -32601
        let req = r#"{"jsonrpc":"2.0","id":42,"method":"unknown/method"}"#;
        let resp = server
            .handle_request_line(req)
            .await
            .expect("Expected error response for unknown method request");
        assert_eq!(resp.id, Some(json!(42)));
        let err = resp.error.expect("Expected error payload");
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found: unknown/method");

        // Notification without id -> returns None
        let notification = r#"{"jsonrpc":"2.0","method":"unknown/notification"}"#;
        let resp_notif = server.handle_request_line(notification).await;
        assert!(resp_notif.is_none());

        // Standard notification returns None
        let init_notif = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        let resp_init = server.handle_request_line(init_notif).await;
        assert!(resp_init.is_none());
    }

    #[tokio::test]
    async fn test_jsonrpc_ping() {
        let server = McpServer::new();
        let req = r#"{"jsonrpc":"2.0","id":7,"method":"ping"}"#;
        let resp = server
            .handle_request_line(req)
            .await
            .expect("Expected ping response");
        assert_eq!(resp.jsonrpc, "2.0");
        assert_eq!(resp.id, Some(json!(7)));
        assert!(resp.error.is_none());
        assert_eq!(resp.result, Some(json!({})));
    }
}
