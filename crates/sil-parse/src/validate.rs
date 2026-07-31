//! PDF path validation for parse.

use std::path::Path;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{DocumentStatus, SourceDocument, SourceId, validate_pdf_path};
use sil_db::SilDb;

use crate::error::ParseError;

/// Validate a path for parsing; map AlreadyParsed when id is in DB.
pub fn validate_for_parse(
    path: &Utf8Path,
    db: &SilDb,
) -> Result<(DocumentStatus, SourceDocument), ParseError> {
    let status = validate_pdf_path(path).map_err(|e| ParseError::InvalidDocument(e.to_string()))?;
    let filename = path
        .file_name()
        .map(str::to_string)
        .unwrap_or_else(|| path.to_string());
    let id = SourceId::from_sources_relative(Utf8Path::new(&filename));
    if status.is_parseable() && db.is_parsed(&id).unwrap_or(false) {
        let mut doc = SourceDocument::new(path.to_path_buf());
        doc.id = id;
        doc.status = Some(DocumentStatus::AlreadyParsed);
        return Ok((DocumentStatus::AlreadyParsed, doc));
    }
    let mut doc = SourceDocument::new(path.to_path_buf());
    doc.id = id;
    doc.status = Some(status);
    Ok((status, doc))
}

/// List PDF files under `sources_dir` that are not yet parsed.
pub fn list_unparsed_pdfs(
    sources_dir: &Utf8Path,
    db: &SilDb,
) -> Result<Vec<Utf8PathBuf>, ParseError> {
    if !sources_dir.exists() {
        return Ok(Vec::new());
    }
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(sources_dir.as_str())
        .map_err(|e| ParseError::Message(format!("read sources/: {e}")))?;
    for ent in entries {
        let ent = ent.map_err(|e| ParseError::Message(e.to_string()))?;
        let path = ent.path();
        if !path.is_file() {
            continue;
        }
        let is_pdf = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| e.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false);
        if !is_pdf {
            continue;
        }
        let utf = Utf8PathBuf::from_path_buf(path)
            .map_err(|_| ParseError::Message("non-UTF8 path in sources/".into()))?;
        let filename = utf.file_name().unwrap_or("").to_string();
        let id = SourceId::new(filename);
        if !db.is_parsed(&id).unwrap_or(false) {
            paths.push(utf);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Minimal valid PDF bytes for fixtures.
pub fn minimal_pdf_bytes() -> &'static [u8] {
    b"%PDF-1.1\n%\xe2\xe3\xcf\xd3\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n"
}

/// Write a fixture PDF to `path`.
pub fn write_fixture_pdf(path: &Path) -> std::io::Result<()> {
    std::fs::write(path, minimal_pdf_bytes())
}
