//! `sil-mcp` — Model Context Protocol (MCP) server for scientist-in-loop.

#![deny(missing_docs)]

/// JSON-RPC 2.0 protocol request, response, and tool schemas.
pub mod protocol;
/// Official `rmcp` protocol service used by the stdio entrypoint.
pub mod sdk;
/// MCP root and caller-path security.
pub mod security;
/// Stdio MCP server handler.
pub mod server;
/// Scientist-in-loop tool registry and invocation.
pub mod tools;

pub use protocol::{
    CallToolResult, Content, JsonRpcError, JsonRpcRequest, JsonRpcResponse, Tool, ToolInputSchema,
};
pub use security::McpContext;
pub use server::McpServer;
pub use tools::{call_tool, list_tools};

/// Convenience helper to start the stdio server using tokio runtime.
pub fn run_stdio_server(quiet: bool, project: Option<camino::Utf8PathBuf>) -> anyhow::Result<()> {
    let context = match project {
        Some(root) => McpContext::from_root(root).map_err(anyhow::Error::msg)?,
        None => McpContext::from_cwd().map_err(anyhow::Error::msg)?,
    };
    if context.discovered_from_cwd {
        eprintln!(
            "⚠ MCP project root discovered from CWD: {} (use --project for installed clients)",
            context.root
        );
    }
    if !quiet {
        eprintln!("⚙ scientist-in-loop MCP server starting over stdio...");
    }
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { sdk::run_stdio(context).await })
}
