//! `sil-mcp` — Model Context Protocol (MCP) server for scientist-in-loop.

#![deny(missing_docs)]

/// JSON-RPC 2.0 protocol request, response, and tool schemas.
pub mod protocol;
/// Stdio MCP server handler.
pub mod server;
/// Scientist-in-loop tool registry and invocation.
pub mod tools;

pub use protocol::{
    CallToolResult, Content, JsonRpcError, JsonRpcRequest, JsonRpcResponse, Tool, ToolInputSchema,
};
pub use server::McpServer;
pub use tools::{call_tool, list_tools};

/// Convenience helper to start the stdio server using tokio runtime.
pub fn run_stdio_server(quiet: bool) -> anyhow::Result<()> {
    if !quiet {
        eprintln!("⚙ scientist-in-loop MCP server starting over stdio...");
    }
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async {
        let stdin = tokio::io::stdin();
        let stdout = tokio::io::stdout();
        let server = McpServer::new();
        server.run(stdin, stdout).await
    })
}
