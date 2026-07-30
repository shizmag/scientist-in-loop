//! Command handler for `sil mcp` — starts the stdio JSON-RPC 2.0 MCP server.

use anyhow::Result;

/// Run the `sil mcp` stdio server.
pub fn run(quiet: bool) -> Result<()> {
    sil_mcp::run_stdio_server(quiet)
}

