//! Single- and multi-PDF parse orchestration.

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{DocumentStatus, SilUi, SourceDocument};
use sil_db::SilDb;

use crate::error::ParseError;
use crate::marker::MarkerRunner;
use crate::validate::validate_for_parse;

/// Result of parsing one PDF.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Source document metadata.
    pub document: SourceDocument,
    /// Extracted plain text / markdown.
    pub content: String,
}

/// Parse one PDF and write into the database.
pub fn parse_one(
    path: &Utf8Path,
    db: &SilDb,
    runner: &dyn MarkerRunner,
    ui: &dyn SilUi,
) -> Result<ParseResult, ParseError> {
    let (status, mut doc) = validate_for_parse(path, db)?;
    match status {
        DocumentStatus::ValidPdf => {}
        DocumentStatus::NotFound => {
            return Err(ParseError::InvalidDocument(format!(
                "file not found: {path}"
            )));
        }
        DocumentStatus::NotPdf => {
            return Err(ParseError::InvalidDocument(format!(
                "not a PDF file: {path}"
            )));
        }
        DocumentStatus::Corrupted => {
            return Err(ParseError::InvalidDocument(format!(
                "corrupted or unreadable PDF: {path}"
            )));
        }
        DocumentStatus::AlreadyParsed => {
            return Err(ParseError::InvalidDocument(format!(
                "already parsed: {} (remove DB row to re-parse)",
                doc.filename
            )));
        }
    }

    ui.info(&format!("Parsing {}", doc.filename));
    let content = runner.parse_pdf(path)?;
    doc.title = content.lines().find_map(|l| {
        let t = l.trim();
        t.strip_prefix("# ").map(str::to_string)
    });
    doc.references_text = extract_references(&content);
    doc.parsed = true;
    doc.status = Some(DocumentStatus::ValidPdf);

    db.upsert_parsed(&doc, &content)
        .map_err(|e| ParseError::Db(e.to_string()))?;

    Ok(ParseResult {
        document: doc,
        content,
    })
}

/// Parse many PDFs with a progress bar; returns (ok, failed) counts and errors.
pub fn parse_many(
    paths: &[Utf8PathBuf],
    db: &SilDb,
    runner: &dyn MarkerRunner,
    ui: &dyn SilUi,
) -> (usize, usize, Vec<(Utf8PathBuf, String)>) {
    let total = paths.len() as u64;
    let mut pb = ui.progress(total, "Parsing PDFs");
    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        pb.set_message(path.file_name().unwrap_or(path.as_str()));
        match parse_one(path, db, runner, ui) {
            Ok(_) => ok += 1,
            Err(e) => {
                failed += 1;
                errors.push((path.clone(), e.to_string()));
            }
        }
        pb.set_position((i as u64) + 1);
    }
    if failed == 0 {
        pb.finish_success(&format!("Parsed {ok} PDF(s)"));
    } else {
        pb.finish_error(&format!("Parsed {ok} PDF(s), {failed} failed"));
    }
    (ok, failed, errors)
}

fn extract_references(content: &str) -> Option<String> {
    let mut references = Vec::new();
    let mut in_refs = false;
    
    for line in content.lines() {
        let t = line.trim();
        if t.starts_with('#') {
            let clean = t.trim_start_matches('#').trim().to_lowercase();
            if clean == "references" || clean == "bibliography" || clean.ends_with(" references") || clean.ends_with(" bibliography") {
                in_refs = true;
                continue;
            } else if in_refs {
                break;
            }
        }
        if in_refs {
            references.push(line);
        }
    }
    
    let joined = references.join("\n").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}
