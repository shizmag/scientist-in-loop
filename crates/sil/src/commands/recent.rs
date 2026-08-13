//! `sil paper recent` — list recently opened scientist-in-loop projects.

use anyhow::Result;
use sil_core::{GlobalSettings, SilUi};

/// List recent projects from global settings.
pub fn run(json: bool, ui: &dyn SilUi) -> Result<()> {
    let settings = GlobalSettings::load_or_default(None);

    if json {
        println!(
            "{}",
            serde_json::to_string_pretty(&settings.recent_projects)?
        );
        return Ok(());
    }

    if settings.recent_projects.is_empty() {
        ui.info("No recent sil projects recorded.");
        return Ok(());
    }

    ui.info("Recent sil projects:");
    for (idx, p) in settings.recent_projects.iter().enumerate() {
        ui.println(&format!("  [{}] {}", idx + 1, p));
    }

    Ok(())
}
