//! `sil-mcp` — Model Context Protocol (MCP) server for scientist-in-loop.

#![deny(missing_docs)]

pub mod protocol;
pub mod server;
pub mod tools;

pub use protocol::{
    CallToolResult, Content, JsonRpcError, JsonRpcRequest, JsonRpcResponse, Tool, ToolInputSchema,
};
pub use server::McpServer;
pub use tools::{call_tool, list_tools};
