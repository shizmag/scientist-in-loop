//! `sil cite` — suggest BibTeX / `\cite{...}` from a source or query.

use anyhow::{Result, bail};
use sil_core::{SilUi, SourceDocument, suggest_from_query, suggest_from_source};
use sil_db::SilDb;

use crate::util::load_project;

/// Suggest a citation artifact from a source id/filename or free-text query.
pub fn run(target: &str, append: bool, promote: bool, json: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, paths) = load_project()?;

    if promote {
        let ctx = sil_app::AppContext::from_root(&root)?;
        let res = sil_app::promote_bib(
            &ctx,
            sil_app::PromoteBib {
                target: target.to_string(),
            },
        )?;
        ui.success(&format!(
            "✓ Promoted entry '{}' in {} (removed % [sil: tui-added])",
            res.cite_key, res.path
        ));
        return Ok(());
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
        let ctx = sil_app::AppContext::from_root(&root)?;
        let res = sil_app::upsert_bib(
            &ctx,
            sil_app::UpsertBib {
                entry: suggestion.bibtex.clone(),
                draft: false,
            },
        )?;
        if res.replaced {
            ui.success(&format!("Updated existing entry in {}", res.path));
        } else {
            ui.success(&format!("Appended entry to {}", res.path));
        }
    }

    if suggestion.cite_key.is_empty() || suggestion.bibtex.trim().is_empty() {
        bail!("internal error: empty citation suggestion");
    }
    Ok(())
}
