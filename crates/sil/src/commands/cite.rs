//! `sil cite` — suggest BibTeX / `\cite{...}` from a source or query.

use anyhow::{Result, bail};
use sil_core::{SilUi, SourceDocument, suggest_from_query, suggest_from_source};
use sil_db::SilDb;

use crate::util::load_project;

/// Suggest a citation artifact from a source id/filename or free-text query.
pub fn run(target: &str, append: bool, json: bool, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let is_filename_target = [".pdf", ".md", ".markdown", ".txt", ".html", ".htm"]
        .iter()
        .any(|ext| target.to_ascii_lowercase().ends_with(ext));

    let suggestion = if let Some(doc) = db
        .list_sources()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .into_iter()
        .find(|d| {
            d.filename == target || d.id.as_str() == target || d.path.as_str().ends_with(target)
        }) {
        suggest_from_source(&doc)
    } else if let Ok(ref_hits) = db.search_references(target, 1)
        && let Some(ref_entry) = ref_hits.first()
    {
        sil_core::suggest_from_reference_entry(ref_entry)
    } else if target.contains(' ') || !is_filename_target {
        // Free-text / search-style query
        suggest_from_query(target)
    } else {
        // Filename not in DB yet — still deterministic from name
        suggest_from_source(&SourceDocument::new(target.into()))
    };

    if json {
        println!("{}", serde_json::to_string_pretty(&suggestion)?);
    } else {
        ui.info(&format!("Citation suggestion for “{target}”"));
        ui.println(&format!("  cite: {}", suggestion.cite_command));
        ui.println(&format!("  key:  {}", suggestion.cite_key));
        ui.muted(&format!("  {}", suggestion.note));
        ui.println("");
        ui.println(&suggestion.bibtex);
    }

    if append {
        let bib_path = paths.join(sil_core::paths::rel::REFERENCES);
        let existing = if bib_path.is_file() {
            std::fs::read_to_string(bib_path.as_str())?
        } else {
            String::new()
        };
        let (updated, replaced) = sil_core::bib::upsert_bib_entry(&existing, &suggestion.bibtex);
        std::fs::write(bib_path.as_str(), updated)?;
        if replaced {
            ui.success(&format!("Updated existing entry in {bib_path}"));
        } else {
            ui.success(&format!("Appended entry to {bib_path}"));
        }
    }

    if suggestion.cite_key.is_empty() || suggestion.bibtex.trim().is_empty() {
        bail!("internal error: empty citation suggestion");
    }
    Ok(())
}
