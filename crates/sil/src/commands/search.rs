//! `sil search`

use anyhow::Result;
use sil_core::SilUi;
use sil_db::SilDb;

use crate::util::load_project;

pub fn run(query: &str, limit: usize, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let hits = db
        .search(query, limit)
        .map_err(|e| anyhow::anyhow!("{e}"))?;
    if hits.is_empty() {
        ui.warn(&format!("No results for “{query}”"));
        return Ok(());
    }
    ui.info(&format!("{} result(s) for “{query}”", hits.len()));
    ui.println("");
    for (i, h) in hits.iter().enumerate() {
        let title = h.title.as_deref().unwrap_or("");
        ui.println(&format!("{}. {} {title}", i + 1, h.filename));
        ui.muted(&format!("   {}", h.snippet.replace('\n', " ")));
    }
    ui.println("");
    Ok(())
}
