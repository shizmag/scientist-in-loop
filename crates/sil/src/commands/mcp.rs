//! Command handler for `sil mcp` and `sil mcp install`.

use crate::cli::McpCmd;
use crate::mcp_install::{InstallOptions, run_installer};
use anyhow::Result;

/// Run the `sil mcp` stdio server or subcommands.
pub fn run(action: Option<McpCmd>, quiet: bool) -> Result<()> {
    match action {
        Some(McpCmd::Install { client, path }) => {
            let options = InstallOptions { client, path };
            run_installer(options)?;
            Ok(())
        }
        None => sil_mcp::run_stdio_server(quiet),
    }
}
