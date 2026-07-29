//! `sil log`

use anyhow::Result;
use sil_core::SilUi;
use sil_git::log_entries;

use crate::util::load_project;

pub fn run(limit: usize, sci_only: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, _paths) = load_project()?;
    let entries = log_entries(&root, limit, sci_only).map_err(|e| anyhow::anyhow!("{e}"))?;
    if entries.is_empty() {
        ui.warn("No matching commits.");
        return Ok(());
    }
    ui.info(&format!(
        "Git log{} (limit {limit})",
        if sci_only { " [Sci-Action]" } else { "" }
    ));
    ui.println("");
    for e in entries {
        let act = e
            .action
            .map(|a| format!("[{}] ", a.as_str()))
            .unwrap_or_default();
        ui.println(&format!("{} {act}{}", e.hash, e.subject));
    }
    ui.println("");
    Ok(())
}
