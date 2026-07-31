//! Source document identity and validation status.

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::error::ValidationError;

/// Opaque identity for a source document (typically a stable relative path key).
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SourceId(String);

impl SourceId {
    /// Create a source id from a string key.
    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    /// Borrow the underlying string.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// Build a source id from a path relative to the sources directory.
    pub fn from_sources_relative(path: &Utf8Path) -> Self {
        Self(path.as_str().replace('\\', "/"))
    }
}

impl fmt::Display for SourceId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl From<String> for SourceId {
    fn from(value: String) -> Self {
        Self(value)
    }
}

impl From<&str> for SourceId {
    fn from(value: &str) -> Self {
        Self(value.to_string())
    }
}

/// Validation / processing status of a document candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// File exists and looks like a valid PDF.
    ValidPdf,
    /// Path does not exist.
    NotFound,
    /// File exists but is not a PDF.
    NotPdf,
    /// Already present and parsed in the database.
    AlreadyParsed,
    /// File looks like a PDF but is corrupted / unreadable.
    Corrupted,
}

impl DocumentStatus {
    /// Human-readable description.
    pub fn message(self) -> &'static str {
        match self {
            Self::ValidPdf => "valid PDF",
            Self::NotFound => "file not found",
            Self::NotPdf => "not a PDF file",
            Self::AlreadyParsed => "already parsed",
            Self::Corrupted => "corrupted or unreadable PDF",
        }
    }

    /// Whether parsing may proceed for this status.
    pub fn is_parseable(self) -> bool {
        matches!(self, Self::ValidPdf)
    }
}

impl fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// A source PDF tracked by the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDocument {
    /// Stable identity.
    pub id: SourceId,
    /// Path relative to project root (or absolute when validated outside project).
    pub path: Utf8PathBuf,
    /// Original filename.
    pub filename: String,
    /// Whether content has been written to SQLite.
    pub parsed: bool,
    /// Last validation result (if known).
    pub status: Option<DocumentStatus>,
    /// Optional title extracted at parse time.
    pub title: Option<String>,
    /// Extracted references/bibliography text.
    pub references_text: Option<String>,
}

/// An individual extracted reference / citation item from a source document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReferenceEntry {
    /// Unique identifier for this reference item.
    pub id: String,
    /// Parent source document ID.
    pub source_id: SourceId,
    /// 1-based index of the reference in the document.
    pub ref_index: usize,
    /// Full unparsed raw citation text.
    pub raw_text: String,
    /// Parsed title if extracted.
    pub title: Option<String>,
    /// Parsed authors string if extracted.
    pub authors: Option<String>,
    /// Parsed publication year if extracted.
    pub year: Option<i32>,
    /// Parsed DOI if extracted.
    pub doi: Option<String>,
}

impl SourceDocument {
    /// Create a new unparsed source document from a path.
    pub fn new(path: Utf8PathBuf) -> Self {
        let filename = path
            .file_name()
            .map(str::to_string)
            .unwrap_or_else(|| path.to_string());
        let id = SourceId::from_sources_relative(Utf8Path::new(&filename));
        Self {
            id,
            path,
            filename,
            parsed: false,
            status: None,
            title: None,
            references_text: None,
        }
    }
}

/// Validate a filesystem path as a PDF candidate (basic magic-byte check).
pub fn validate_pdf_path(path: &Utf8Path) -> Result<DocumentStatus, ValidationError> {
    if !path.exists() {
        return Ok(DocumentStatus::NotFound);
    }
    if !path.is_file() {
        return Ok(DocumentStatus::NotPdf);
    }
    let bytes = std::fs::read(path).map_err(|e| ValidationError::Io {
        path: path.to_string(),
        message: e.to_string(),
    })?;
    if bytes.len() < 5 {
        return Ok(DocumentStatus::Corrupted);
    }
    // PDF magic: %PDF-
    if &bytes[0..5] != b"%PDF-" {
        // Extension hint
        if path
            .extension()
            .map(|e| e.eq_ignore_ascii_case("pdf"))
            .unwrap_or(false)
        {
            return Ok(DocumentStatus::Corrupted);
        }
        return Ok(DocumentStatus::NotPdf);
    }
    Ok(DocumentStatus::ValidPdf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn source_id_display() {
        let id = SourceId::new("paper.pdf");
        assert_eq!(id.as_str(), "paper.pdf");
        assert_eq!(id.to_string(), "paper.pdf");
    }

    #[test]
    fn validate_missing() {
        let status = validate_pdf_path(Utf8Path::new("/no/such/file.pdf")).unwrap();
        assert_eq!(status, DocumentStatus::NotFound);
    }

    #[test]
    fn validate_valid_pdf_magic() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"%PDF-1.4\n%sil-test\n").unwrap();
        let path = Utf8PathBuf::from_path_buf(f.path().to_path_buf()).unwrap();
        assert_eq!(validate_pdf_path(&path).unwrap(), DocumentStatus::ValidPdf);
    }

    #[test]
    fn validate_not_pdf() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"hello world").unwrap();
        let path = Utf8PathBuf::from_path_buf(f.path().to_path_buf()).unwrap();
        assert_eq!(validate_pdf_path(&path).unwrap(), DocumentStatus::NotPdf);
    }

    #[test]
    fn document_status_parseable() {
        assert!(DocumentStatus::ValidPdf.is_parseable());
        assert!(!DocumentStatus::AlreadyParsed.is_parseable());
        assert!(!DocumentStatus::NotFound.is_parseable());
        assert!(!DocumentStatus::NotPdf.is_parseable());
        assert!(!DocumentStatus::Corrupted.is_parseable());
    }

    #[test]
    fn document_status_messages() {
        for s in [
            DocumentStatus::ValidPdf,
            DocumentStatus::NotFound,
            DocumentStatus::NotPdf,
            DocumentStatus::AlreadyParsed,
            DocumentStatus::Corrupted,
        ] {
            assert!(!s.message().is_empty());
            assert_eq!(s.to_string(), s.message());
        }
    }

    #[test]
    fn source_document_new_sets_filename() {
        let doc = SourceDocument::new("sources/foo.pdf".into());
        assert_eq!(doc.filename, "foo.pdf");
        assert!(!doc.parsed);
        assert!(doc.status.is_none());
    }

    #[test]
    fn source_id_from_str_and_string() {
        let a: SourceId = "a.pdf".into();
        let b = SourceId::from("b.pdf".to_string());
        assert_eq!(a.as_str(), "a.pdf");
        assert_eq!(b.as_str(), "b.pdf");
    }

    #[test]
    fn validate_corrupted_short_file() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"x").unwrap();
        let path = Utf8PathBuf::from_path_buf(f.path().to_path_buf()).unwrap();
        // not PDF magic → NotPdf (or Corrupted if .pdf extension)
        let status = validate_pdf_path(&path).unwrap();
        assert!(matches!(
            status,
            DocumentStatus::NotPdf | DocumentStatus::Corrupted
        ));
    }

    #[test]
    fn validate_corrupted_pdf_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.pdf");
        std::fs::write(&path, b"%PD").unwrap(); // too short / wrong
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        let status = validate_pdf_path(&path).unwrap();
        assert!(matches!(
            status,
            DocumentStatus::Corrupted | DocumentStatus::NotPdf
        ));
    }

    #[test]
    fn validate_directory_is_not_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let status = validate_pdf_path(&path).unwrap();
        assert_eq!(status, DocumentStatus::NotPdf);
    }

    #[test]
    fn validate_empty_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.pdf");
        std::fs::write(&path, b"").unwrap();
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        let status = validate_pdf_path(&path).unwrap();
        assert!(matches!(
            status,
            DocumentStatus::Corrupted | DocumentStatus::NotPdf
        ));
    }

    #[test]
    fn source_id_normalizes_backslashes() {
        let id = SourceId::from_sources_relative(Utf8Path::new("sub\\file.pdf"));
        assert!(!id.as_str().contains('\\'));
        assert!(id.as_str().contains("file.pdf") || id.as_str().contains('/'));
    }

    #[test]
    fn source_document_equality_on_id_path() {
        let a = SourceDocument::new("a.pdf".into());
        let b = SourceDocument::new("a.pdf".into());
        assert_eq!(a.id, b.id);
        assert_eq!(a.filename, b.filename);
    }
}
