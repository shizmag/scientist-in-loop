//! `sil cite` — suggest BibTeX / `\cite{...}` from a source or query.

use anyhow::{Result, bail};
use sil_core::{SilUi, SourceDocument, suggest_from_query, suggest_from_source};
use sil_db::SilDb;

use crate::util::load_project;

/// Suggest a citation artifact from a source id/filename or free-text query.
pub fn run(target: &str, append: bool, promote: bool, json: bool, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;

    if promote {
        let bib_path = paths.join(sil_core::paths::rel::REFERENCES);
        if !bib_path.is_file() {
            bail!("references.bib not found at {bib_path}");
        }
        let current = std::fs::read_to_string(bib_path.as_str())?;
        let target_info = sil_core::BibEntryInfo {
            cite_key: Some(target.to_string()),
            title: Some(target.to_string()),
            doi: Some(target.to_string()),
            arxiv_id: Some(target.to_string()),
            is_incomplete: false,
        };
        let mut blocks = sil_core::parse_bib_blocks(&current);
        let mut promoted_key = None;
        for block in &mut blocks {
            let block_info = sil_core::extract_bib_entry_info(block);
            if sil_core::is_same_paper(&block_info, &target_info)
                || block_info.cite_key.as_deref().unwrap_or("").to_lowercase()
                    == target.to_lowercase()
            {
                let cite_key = block_info.cite_key.as_deref().unwrap_or(target).to_string();
                *block = sil_core::unmark_tui_added_bib_entry(block);
                promoted_key = Some(cite_key);
                break;
            }
        }

        if let Some(key) = promoted_key {
            let updated = if blocks.is_empty() {
                String::new()
            } else {
                blocks.join("\n\n") + "\n"
            };
            sil_core::write_atomic_str(&bib_path, &updated)?;
            ui.success(&format!(
                "✓ Promoted entry '{key}' in {bib_path} (removed % [sil: tui-added])"
            ));
            return Ok(());
        } else {
            bail!("No entry matching '{target}' found in {bib_path} to promote");
        }
    }

    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;

    let is_filename_target = [".pdf", ".md", ".markdown", ".txt", ".html", ".htm"]
        .iter()
        .any(|ext| target.to_ascii_lowercase().ends_with(ext));

    let (suggestion, official_resolution) = if let Some(doc) = db
        .list_sources()
        .map_err(|e| anyhow::anyhow!("{e}"))?
        .into_iter()
        .find(|d| {
            d.filename == target || d.id.as_str() == target || d.path.as_str().ends_with(target)
        }) {
        let res = sil_parse::resolve_official_bibtex_for_source(&doc);
        match res {
            sil_parse::SourceBibResolution::Resolved(ref bib) => {
                let info = sil_core::extract_bib_entry_info(bib);
                let cite_key = info.cite_key.unwrap_or_else(|| {
                    sil_core::slug_cite_key(doc.title.as_deref().unwrap_or(&doc.filename))
                });
                let sug = sil_core::BibSuggestion {
                    cite_key: cite_key.clone(),
                    cite_command: sil_core::format_cite_command(&cite_key),
                    bibtex: bib.clone(),
                    note: "Official metadata resolved via DOI/Crossref/arXiv API".to_string(),
                };
                (sug, Some(res))
            }
            sil_parse::SourceBibResolution::Failed(_) => (suggest_from_source(&doc), Some(res)),
        }
    } else if let Ok(ref_hits) = db.search_references(target, 1)
        && let Some(ref_entry) = ref_hits.first()
    {
        (sil_core::suggest_from_reference_entry(ref_entry), None)
    } else if target.contains(' ') || !is_filename_target {
        // Free-text / search-style query
        (suggest_from_query(target), None)
    } else {
        // Filename not in DB yet — attempt official resolve on new SourceDocument struct
        let doc = SourceDocument::new(target.into());
        let res = sil_parse::resolve_official_bibtex_for_source(&doc);
        match res {
            sil_parse::SourceBibResolution::Resolved(ref bib) => {
                let info = sil_core::extract_bib_entry_info(bib);
                let cite_key = info
                    .cite_key
                    .unwrap_or_else(|| sil_core::slug_cite_key(&doc.filename));
                let sug = sil_core::BibSuggestion {
                    cite_key: cite_key.clone(),
                    cite_command: sil_core::format_cite_command(&cite_key),
                    bibtex: bib.clone(),
                    note: "Official metadata resolved via DOI/Crossref/arXiv API".to_string(),
                };
                (sug, Some(res))
            }
            sil_parse::SourceBibResolution::Failed(_) => (suggest_from_source(&doc), Some(res)),
        }
    };

    if !json && let Some(sil_parse::SourceBibResolution::Failed(ref reason)) = official_resolution {
        ui.warn(&format!("⚠ Could not resolve official metadata: {reason}"));
    }

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
        sil_core::write_atomic_str(&bib_path, &updated)?;
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
