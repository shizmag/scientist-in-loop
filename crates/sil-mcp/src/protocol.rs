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
