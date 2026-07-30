//! `sil settings` command handler launching Ratatui interface.

use anyhow::Result;
use sil_core::paths::project_root_from_cwd;

pub fn run() -> Result<()> {
    let project_root = project_root_from_cwd().ok();
    sil_tui::run_tui(project_root)
}
