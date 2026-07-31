//! JSON-RPC 2.0 and MCP protocol data structures.

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 Request message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcRequest {
    /// JSON-RPC protocol version ("2.0").
    pub jsonrpc: String,
    /// Request identifier (null for notifications).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub id: Option<serde_json::Value>,
    /// Remote method name.
    pub method: String,
    /// Optional parameters payload.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub params: Option<serde_json::Value>,
}

/// JSON-RPC 2.0 Response message.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcResponse {
    /// JSON-RPC protocol version ("2.0").
    pub jsonrpc: String,
    /// Request identifier matching request id.
    #[serde(default)]
    pub id: Option<serde_json::Value>,
    /// Result payload on success.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error payload on failure.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a successful response with standard jsonrpc "2.0".
    pub fn success(id: Option<serde_json::Value>, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response with standard jsonrpc "2.0".
    pub fn error(id: Option<serde_json::Value>, code: i32, message: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message: message.into(),
                data: None,
            }),
        }
    }
}

/// JSON-RPC 2.0 Error struct.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JsonRpcError {
    /// Numeric error code.
    pub code: i32,
    /// Human readable error message.
    pub message: String,
    /// Optional error metadata or stack trace data.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

/// MCP Tool definition descriptor.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Tool {
    /// Unique tool name.
    pub name: String,
    /// Descriptive summary of what the tool does.
    pub description: String,
    /// Input schema describing parameters accepted by the tool.
    #[serde(rename = "inputSchema")]
    pub input_schema: ToolInputSchema,
}

/// JSON schema for tool inputs.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ToolInputSchema {
    /// Schema type, typically "object".
    #[serde(rename = "type")]
    pub schema_type: String,
    /// Map of parameter names to their json schemas.
    #[serde(default)]
    pub properties: serde_json::Value,
    /// List of required property names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub required: Vec<String>,
}

impl ToolInputSchema {
    /// Helper to create a standard object schema.
    pub fn object(properties: serde_json::Value, required: Vec<&str>) -> Self {
        Self {
            schema_type: "object".to_string(),
            properties,
            required: required.into_iter().map(String::from).collect(),
        }
    }
}

/// Result returned from a tool call execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct CallToolResult {
    /// Content items produced by tool execution.
    pub content: Vec<Content>,
    /// Optional flag indicating if execution encountered an error.
    #[serde(rename = "isError", skip_serializing_if = "Option::is_none")]
    pub is_error: Option<bool>,
}

impl CallToolResult {
    /// Create a text content result.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text { text: text.into() }],
            is_error: None,
        }
    }

    /// Create an error text result.
    pub fn error(message: impl Into<String>) -> Self {
        Self {
            content: vec![Content::Text {
                text: message.into(),
            }],
            is_error: Some(true),
        }
    }
}

/// Individual content block in MCP tool results.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum Content {
    /// Text content block.
    #[serde(rename = "text")]
    Text {
        /// Text content string.
        text: String,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jsonrpc_request_serde() {
        let req = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: Some(json!(1)),
            method: "ping".to_string(),
            params: Some(json!({"key": "val"})),
        };
        let serialized = serde_json::to_string(&req).unwrap();
        assert!(serialized.contains(r#""jsonrpc":"2.0""#));
        assert!(serialized.contains(r#""id":1"#));
        assert!(serialized.contains(r#""method":"ping""#));

        let deserialized: JsonRpcRequest = serde_json::from_str(&serialized).unwrap();
        assert_eq!(req, deserialized);

        let req_no_id_no_params = JsonRpcRequest {
            jsonrpc: "2.0".to_string(),
            id: None,
            method: "notifications/initialized".to_string(),
            params: None,
        };
        let ser_no_id = serde_json::to_string(&req_no_id_no_params).unwrap();
        assert!(!ser_no_id.contains("id"));
        assert!(!ser_no_id.contains("params"));
    }

    #[test]
    fn test_jsonrpc_response_success_and_error() {
        let succ = JsonRpcResponse::success(Some(json!(42)), json!({"status": "ok"}));
        assert_eq!(succ.jsonrpc, "2.0");
        assert_eq!(succ.id, Some(json!(42)));
        assert_eq!(succ.result, Some(json!({"status": "ok"})));
        assert!(succ.error.is_none());

        let succ_json = serde_json::to_string(&succ).unwrap();
        assert!(!succ_json.contains("error"));

        let err = JsonRpcResponse::error(Some(json!("req_2")), -32601, "Method not found");
        assert_eq!(err.jsonrpc, "2.0");
        assert_eq!(err.id, Some(json!("req_2")));
        assert!(err.result.is_none());
        assert_eq!(
            err.error,
            Some(JsonRpcError {
                code: -32601,
                message: "Method not found".to_string(),
                data: None,
            })
        );

        let err_json = serde_json::to_string(&err).unwrap();
        assert!(!err_json.contains("result"));
    }

    #[test]
    fn test_jsonrpc_error_data() {
        let err_with_data = JsonRpcError {
            code: -32000,
            message: "Custom error".to_string(),
            data: Some(json!({"trace": "stacktrace..."})),
        };
        let ser = serde_json::to_string(&err_with_data).unwrap();
        assert!(ser.contains("trace"));
        let de: JsonRpcError = serde_json::from_str(&ser).unwrap();
        assert_eq!(err_with_data, de);
    }

    #[test]
    fn test_tool_input_schema_object() {
        let schema = ToolInputSchema::object(json!({"param1": {"type": "string"}}), vec!["param1"]);
        assert_eq!(schema.schema_type, "object");
        assert_eq!(schema.required, vec!["param1"]);

        let ser = serde_json::to_string(&schema).unwrap();
        assert!(ser.contains(r#""type":"object""#));
        assert!(ser.contains(r#""required":["param1"]"#));
    }

    #[test]
    fn test_call_tool_result() {
        let res_text = CallToolResult::text("hello world");
        assert_eq!(
            res_text.content,
            vec![Content::Text {
                text: "hello world".to_string(),
            }]
        );
        assert_eq!(res_text.is_error, None);

        let res_err = CallToolResult::error("something broke");
        assert_eq!(res_err.is_error, Some(true));

        let ser_text = serde_json::to_string(&res_text).unwrap();
        assert!(ser_text.contains(r#""type":"text""#));
        assert!(ser_text.contains("hello world"));
        assert!(!ser_text.contains("isError"));
    }
}
