//! Single- and multi-PDF parse orchestration.

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{DocumentStatus, SilUi, SourceDocument, SourceKind};
use sil_db::SilDb;

use crate::error::ParseError;
use crate::marker::MarkerRunner;
use crate::validate::validate_for_parse;

/// Result of parsing one source document.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Source document metadata.
    pub document: SourceDocument,
    /// Extracted plain text / markdown.
    pub content: String,
    /// Time taken for extraction and processing.
    pub duration: std::time::Duration,
    /// Total reference entries extracted.
    pub reference_count: usize,
}

fn extract_heading_title(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(h1) = trimmed.strip_prefix("# ") {
            let t = h1.trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            let heading = trimmed.trim_start_matches('#').trim();
            if !heading.is_empty() {
                return Some(heading.to_string());
            }
        }
    }
    None
}

/// Parse one source document and write into the database.
pub fn parse_one(
    path: &Utf8Path,
    db: &SilDb,
    runner: &dyn MarkerRunner,
    ui: &dyn SilUi,
) -> Result<ParseResult, ParseError> {
    let start_time = std::time::Instant::now();
    let (status, mut doc) = validate_for_parse(path, db)?;
    match status {
        DocumentStatus::Valid(kind) => {
            doc.kind = kind;
        }
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
        DocumentStatus::UnsupportedFormat => {
            return Err(ParseError::InvalidDocument(format!(
                "unsupported format: {path}"
            )));
        }
        DocumentStatus::Corrupted => {
            return Err(ParseError::InvalidDocument(format!(
                "corrupted or unreadable document: {path}"
            )));
        }
        DocumentStatus::AlreadyParsed => {
            return Err(ParseError::InvalidDocument(format!(
                "already parsed: {} (remove DB row to re-parse)",
                doc.filename
            )));
        }
    }

    let mut spin = ui.spinner(&format!("Extracting text: {}", doc.filename));
    let content = match doc.kind {
        SourceKind::Pdf => match runner.parse_pdf(path) {
            Ok(c) => {
                spin.finish_success(&format!("Extracted {}", doc.filename));
                c
            }
            Err(e) => {
                spin.finish_error(&format!("Failed extracting {}", doc.filename));
                return Err(e);
            }
        },
        _ => match std::fs::read_to_string(path) {
            Ok(c) => {
                spin.finish_success(&format!("Read {}", doc.filename));
                c
            }
            Err(e) => {
                spin.finish_error(&format!("Failed reading {}", doc.filename));
                return Err(ParseError::InvalidDocument(format!("failed to read text content of {path}: {e}")));
            }
        },
    };

    let extracted_doi = sil_regex::extract_doi(&content);
    let mut hydrated = false;

    if let Some(ref doi) = extracted_doi {
        if let Ok(Some(pub_item)) = crate::journal_digest::fetch_work_by_doi(doi) {
            doc.doi = pub_item.doi.or(extracted_doi.clone());
            if !pub_item.title.is_empty() {
                doc.title = Some(pub_item.title);
            }
            if !pub_item.authors.is_empty() {
                doc.authors = Some(pub_item.authors);
            }
            if let Some(y) = pub_item.year {
                doc.year = Some(y as i32);
            }
            if !pub_item.journal.is_empty() {
                doc.venue = Some(pub_item.journal);
            }
            if !pub_item.abstract_text.is_empty() {
                doc.abstract_text = Some(pub_item.abstract_text);
            }
            hydrated = true;
        }
    }

    if !hydrated {
        if doc.doi.is_none() {
            doc.doi = extracted_doi;
        }
        if doc.title.is_none() {
            doc.title = sil_regex::extract_quoted_title(&content)
                .or_else(|| extract_heading_title(&content))
                .or_else(|| path.file_stem().map(|s| s.to_string()));
        }
    }

    doc.references_text = crate::references::extract_references_block(&content);
    doc.parsed = true;
    doc.status = Some(DocumentStatus::Valid(doc.kind));

    db.upsert_parsed(&doc, &content)
        .map_err(|e| ParseError::Db(e.to_string()))?;

    let mut reference_count = 0;
    if let Some(ref raw_block) = doc.references_text {
        let entries = crate::references::parse_reference_entries(&doc.id, raw_block);
        reference_count = entries.len();
        if !entries.is_empty() {
            db.save_source_references(&entries)
                .map_err(|e| ParseError::Db(e.to_string()))?;
        }
    }

    let duration = start_time.elapsed();

    Ok(ParseResult {
        document: doc,
        content,
        duration,
        reference_count,
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


