//! `sil-tui` binary entry point.

use anyhow::Result;
use sil_core::paths::project_root_from_cwd;
use sil_tui::run_tui;

fn main() -> Result<()> {
    let project_root = project_root_from_cwd().ok();
    run_tui(project_root)
}
