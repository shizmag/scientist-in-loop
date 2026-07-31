//! `sil digest` — fetch top peer-reviewed journal publication digest.

#![allow(clippy::collapsible_if)]

use anyhow::Result;

use sil_core::SilUi;
use sil_parse::fetch_journal_publications;

use crate::util::load_project;

/// Fetch top journal publications matching query.
pub fn run(query: &str, limit: usize, ui: &dyn SilUi) -> Result<()> {
    ui.info(&format!(
        "Fetching top journal publication digest for '{query}' (max {limit})..."
    ));

    let script_path = camino::Utf8Path::new("python/fetch_journal_digest.py");
    let items = fetch_journal_publications(query, limit, Some(script_path), None)?;

    if items.is_empty() {
        ui.warn("No top journal publications retrieved (or script returned empty list).");
        return Ok(());
    }

    // Store in SQLite if inside a project
    if let Ok((_root, _config, paths)) = load_project() {
        if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
            for item in &items {
                let _ = db.save_journal_publication(item);
            }
        }
    }

    ui.println("");
    ui.success(&format!("Top Journal Publications ({})", items.len()));
    ui.println("─────────────────────────────────────────────────────────────");

    for (idx, item) in items.iter().enumerate() {
        let yr = item.year.map(|y| format!(" ({y})")).unwrap_or_default();
        ui.println(&format!(
            "{}. [{}] {}{}",
            idx + 1,
            item.journal,
            item.title,
            yr
        ));
        ui.println(&format!("   Authors: {}", item.authors));
        if let Some(doi) = &item.doi {
            ui.println(&format!("   DOI: {doi}"));
        }
        ui.println(&format!("   URL: {}", item.url));
        ui.println("");
    }

    Ok(())
}
