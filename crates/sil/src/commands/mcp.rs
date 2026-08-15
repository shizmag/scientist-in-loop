//! Command handler for `sil mcp` and `sil mcp install`.

use crate::cli::McpCmd;
use crate::mcp_install::{InstallOptions, install, status, uninstall};
use anyhow::Result;

/// Run the `sil mcp` stdio server or subcommands.
pub fn run(
    action: Option<McpCmd>,
    quiet: bool,
    project: Option<camino::Utf8PathBuf>,
) -> Result<()> {
    match action {
        Some(McpCmd::Install {
            client,
            path,
            project: install_project,
            hook,
        }) => {
            let options = InstallOptions {
                client,
                path,
                project: install_project.or(project.clone().map(|path| path.into_std_path_buf())),
                hook,
            };
            install(options)?;
            Ok(())
        }
        Some(McpCmd::Status { client, path }) => {
            let result = status(InstallOptions {
                client,
                path,
                project: project.clone().map(|path| path.into_std_path_buf()),
                hook: false,
            })?;
            println!(
                "{}: {}",
                result.path.display(),
                if result.installed {
                    "installed"
                } else {
                    "not installed"
                }
            );
            Ok(())
        }
        Some(McpCmd::Uninstall { client, path }) => {
            uninstall(InstallOptions {
                client,
                path,
                project: project.map(|path| path.into_std_path_buf()),
                hook: false,
            })?;
            Ok(())
        }
        None => sil_mcp::run_stdio_server(quiet, project),
    }
}
