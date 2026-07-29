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
    }
}
