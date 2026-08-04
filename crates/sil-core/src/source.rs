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

/// Kind of source document tracked by the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceKind {
    /// PDF document.
    Pdf,
    /// HTML document.
    Html,
    /// Markdown document.
    Markdown,
    /// Plain text document.
    Text,
    /// Code source file.
    Code,
    /// Dataset file (CSV, JSON, etc.).
    Dataset,
    /// Unknown or unspecified source document format.
    Unknown,
}

impl SourceKind {
    /// Guess source kind from file path extension.
    pub fn from_path(path: &Utf8Path) -> Self {
        match path
            .extension()
            .map(|ext| ext.to_ascii_lowercase())
            .as_deref()
        {
            Some("pdf") => Self::Pdf,
            Some("md" | "markdown") => Self::Markdown,
            Some("txt") => Self::Text,
            Some("html" | "htm") => Self::Html,
            Some("rs" | "py" | "js" | "ts" | "c" | "cpp" | "h" | "go" | "java" | "sh") => {
                Self::Code
            }
            Some("csv" | "json" | "parquet") => Self::Dataset,
            _ => Self::Pdf,
        }
    }
}

impl fmt::Display for SourceKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Pdf => write!(f, "pdf"),
            Self::Html => write!(f, "html"),
            Self::Markdown => write!(f, "markdown"),
            Self::Text => write!(f, "text"),
            Self::Code => write!(f, "code"),
            Self::Dataset => write!(f, "dataset"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParseSourceKindError;

impl fmt::Display for ParseSourceKindError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "invalid source kind")
    }
}

impl std::error::Error for ParseSourceKindError {}

impl std::str::FromStr for SourceKind {
    type Err = ParseSourceKindError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "pdf" => Ok(Self::Pdf),
            "html" | "htm" => Ok(Self::Html),
            "markdown" | "md" => Ok(Self::Markdown),
            "text" | "txt" => Ok(Self::Text),
            "code" => Ok(Self::Code),
            "dataset" => Ok(Self::Dataset),
            "unknown" => Ok(Self::Unknown),
            _ => Err(ParseSourceKindError),
        }
    }
}

/// Validation / processing status of a document candidate.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DocumentStatus {
    /// File exists and is a valid format.
    Valid(SourceKind),
    /// Path does not exist.
    NotFound,
    /// File exists but is not a PDF (kept for backward compatibility).
    NotPdf,
    /// Format not supported for parsing.
    UnsupportedFormat,
    /// Already present and parsed in the database.
    AlreadyParsed,
    /// File looks like a known format but is corrupted / unreadable.
    Corrupted,
}

impl DocumentStatus {
    /// Backward compatibility alias for `DocumentStatus::Valid(SourceKind::Pdf)`.
    #[allow(non_upper_case_globals)]
    pub const ValidPdf: DocumentStatus = DocumentStatus::Valid(SourceKind::Pdf);

    /// Human-readable description.
    pub fn message(self) -> &'static str {
        match self {
            Self::Valid(SourceKind::Pdf) => "valid PDF",
            Self::Valid(SourceKind::Html) => "valid HTML document",
            Self::Valid(SourceKind::Markdown) => "valid Markdown document",
            Self::Valid(SourceKind::Text) => "valid text document",
            Self::Valid(SourceKind::Code) => "valid code source",
            Self::Valid(SourceKind::Dataset) => "valid dataset",
            Self::Valid(SourceKind::Unknown) => "valid document",
            Self::NotFound => "file not found",
            Self::NotPdf => "not a PDF file",
            Self::UnsupportedFormat => "unsupported format",
            Self::AlreadyParsed => "already parsed",
            Self::Corrupted => "corrupted or unreadable document",
        }
    }

    /// Whether parsing may proceed for this status.
    pub fn is_parseable(self) -> bool {
        matches!(self, Self::Valid(_))
    }
}

impl fmt::Display for DocumentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.message())
    }
}

/// A source document tracked by the project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceDocument {
    /// Stable identity.
    pub id: SourceId,
    /// Path relative to project root (or absolute when validated outside project).
    pub path: Utf8PathBuf,
    /// Original filename.
    pub filename: String,
    /// Kind of source document (PDF, Markdown, HTML, etc.).
    pub kind: SourceKind,
    /// Whether content has been written to SQLite.
    pub parsed: bool,
    /// Last validation result (if known).
    pub status: Option<DocumentStatus>,
    /// Optional title extracted at parse time.
    pub title: Option<String>,
    /// Optional authors.
    pub authors: Option<String>,
    /// Optional abstract text.
    pub abstract_text: Option<String>,
    /// Optional DOI.
    pub doi: Option<String>,
    /// Optional publication year.
    pub year: Option<i32>,
    /// Optional venue/journal.
    pub venue: Option<String>,
    /// Extracted references/bibliography text.
    pub references_text: Option<String>,
}

impl SourceDocument {
    /// Return true if source doc has resolvable identifiers (DOI, arXiv ID, or non-empty title) suitable for network hydration.
    pub fn should_attempt_metadata_fetch(&self) -> bool {
        let has_doi = self.doi.as_ref().is_some_and(|s| !s.trim().is_empty());
        let has_title = self.title.as_ref().is_some_and(|t| !t.trim().is_empty());
        let has_arxiv = self.filename.to_lowercase().contains("arxiv")
            || self.doi.as_deref().is_some_and(|d| d.to_lowercase().contains("arxiv"))
            || self.title.as_deref().is_some_and(|t| t.to_lowercase().contains("arxiv"));
        has_doi || has_title || has_arxiv
    }
}

/// Standalone helper to check if a `ReferenceEntry` has resolvable identifiers (DOI, arXiv ID, or title).
pub fn should_attempt_metadata_fetch(entry: &ReferenceEntry) -> bool {
    entry.should_attempt_metadata_fetch()
}

/// Standalone helper to check if a `SourceDocument` has resolvable identifiers (DOI, arXiv ID, or title).
pub fn should_attempt_metadata_fetch_source(doc: &SourceDocument) -> bool {
    doc.should_attempt_metadata_fetch()
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
    /// Parsed venue (journal or conference) if extracted.
    pub venue: Option<String>,
    /// Parsed DOI if extracted.
    pub doi: Option<String>,
    /// Parsed arXiv ID if extracted.
    pub arxiv_id: Option<String>,
    /// Parsed URL if extracted.
    pub url: Option<String>,
}

impl ReferenceEntry {
    /// Return true if entry has resolvable identifiers (DOI, arXiv ID, or non-empty title) suitable for network hydration.
    pub fn should_attempt_metadata_fetch(&self) -> bool {
        let has_doi = self.doi.as_ref().is_some_and(|s| !s.trim().is_empty());
        let has_arxiv = self.arxiv_id.as_ref().is_some_and(|s| !s.trim().is_empty());
        let has_title = self.title.as_ref().is_some_and(|t| !t.trim().is_empty());
        has_doi || has_arxiv || has_title
    }

    /// Format the reference as an `@article` or `@misc` BibTeX string.
    pub fn to_bibtex(&self) -> String {
        let title_or_raw = self.title.as_deref().unwrap_or(&self.raw_text);
        let cite_key = crate::bib::slug_cite_key(title_or_raw);
        let author = self.authors.as_deref().unwrap_or("Unknown");
        let journal = self.venue.as_deref().unwrap_or("Unknown");
        let year = self
            .year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "n.d.".to_string());

        let entry_type = if self.venue.is_none() && (self.arxiv_id.is_some() || self.url.is_some())
        {
            "@misc"
        } else {
            "@article"
        };

        let mut fields = vec![
            format!("  title={{{}}}", title_or_raw),
            format!("  author={{{}}}", author),
            format!("  journal={{{}}}", journal),
            format!("  year={{{}}}", year),
            "  note={unproved, incomplete}".to_string(),
        ];
        if let Some(doi) = &self.doi {
            fields.push(format!("  doi={{{}}}", doi));
        }
        if let Some(arxiv_id) = &self.arxiv_id {
            fields.push(format!("  eprint={{{}}}", arxiv_id));
            fields.push("  archivePrefix={arXiv}".to_string());
        }
        if let Some(url) = &self.url {
            fields.push(format!("  url={{{}}}", url));
        }
        let body = fields.join(",\n");
        format!("% [status: unproved, incomplete]\n{entry_type}{{{cite_key},\n{body}\n}}\n")
    }
}

impl SourceDocument {
    /// Create a new unparsed source document from a path.
    pub fn new(path: Utf8PathBuf) -> Self {
        let filename = path
            .file_name()
            .map(str::to_string)
            .unwrap_or_else(|| path.to_string());
        let id = SourceId::from_sources_relative(Utf8Path::new(&filename));
        let kind = SourceKind::from_path(Utf8Path::new(&filename));
        Self {
            id,
            path,
            filename,
            kind,
            parsed: false,
            status: None,
            title: None,
            authors: None,
            abstract_text: None,
            doi: None,
            year: None,
            venue: None,
            references_text: None,
        }
    }
}

/// Validate a filesystem path as a source document candidate.
pub fn probe_source(path: &Utf8Path) -> Result<DocumentStatus, ValidationError> {
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
    if bytes.len() >= 5 && &bytes[0..5] == b"%PDF-" {
        return Ok(DocumentStatus::Valid(SourceKind::Pdf));
    }
    let ext = path.extension().map(|e| e.to_ascii_lowercase());
    match ext.as_deref() {
        Some("pdf") => Ok(DocumentStatus::Corrupted),
        Some("md" | "markdown") => {
            if std::str::from_utf8(&bytes).is_ok() {
                Ok(DocumentStatus::Valid(SourceKind::Markdown))
            } else {
                Ok(DocumentStatus::Corrupted)
            }
        }
        Some("txt") => {
            if std::str::from_utf8(&bytes).is_ok() {
                Ok(DocumentStatus::Valid(SourceKind::Text))
            } else {
                Ok(DocumentStatus::Corrupted)
            }
        }
        Some("html" | "htm") => {
            if std::str::from_utf8(&bytes).is_ok() {
                Ok(DocumentStatus::Valid(SourceKind::Html))
            } else {
                Ok(DocumentStatus::Corrupted)
            }
        }
        _ => Ok(DocumentStatus::UnsupportedFormat),
    }
}

/// Validate a filesystem path as a PDF candidate (backward compatibility wrapper for probe_source).
pub fn validate_pdf_path(path: &Utf8Path) -> Result<DocumentStatus, ValidationError> {
    probe_source(path)
}

/// Strip LaTeX comments, macros, and markup to isolate clean prose for text embedding.
pub fn strip_latex_for_embed(tex: &str) -> String {
    let mut out = String::with_capacity(tex.len());
    for line in tex.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') {
            continue;
        }
        let line_without_comment = if let Some(idx) = find_unescaped_percent(line) {
            &line[..idx]
        } else {
            line
        };

        let clean = strip_latex_commands(line_without_comment);
        if !clean.trim().is_empty() {
            out.push_str(clean.trim());
            out.push(' ');
        }
    }
    out.trim().to_string()
}

fn find_unescaped_percent(line: &str) -> Option<usize> {
    let mut prev_backslash = false;
    for (i, ch) in line.char_indices() {
        if ch == '%' && !prev_backslash {
            return Some(i);
        }
        prev_backslash = ch == '\\' && !prev_backslash;
    }
    None
}

fn strip_latex_commands(text: &str) -> String {
    let mut res = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\\' {
            let mut cmd = String::new();
            while let Some(&next_c) = chars.peek() {
                if next_c.is_alphabetic() {
                    cmd.push(next_c);
                    chars.next();
                } else {
                    break;
                }
            }
            if cmd == "cite"
                || cmd == "ref"
                || cmd == "label"
                || cmd == "bibliography"
                || cmd == "bibliographystyle"
            {
                if let Some(&'{') = chars.peek() {
                    chars.next();
                    let mut depth = 1;
                    while depth > 0 && let Some(ch) = chars.next() {
                        if ch == '{' {
                            depth += 1;
                        } else if ch == '}' {
                            depth -= 1;
                        }
                    }
                }
            } else if let Some(&' ') = chars.peek() {
                chars.next();
            }
        } else if c != '{' && c != '}' {
            res.push(c);
        }
    }
    res
}

/// Format reference entry into a single dense string representation suitable for text embedding.
pub fn ref_text_for_embed(entry: &ReferenceEntry) -> String {
    let mut parts = Vec::new();
    if let Some(ref title) = entry.title {
        let t = title.trim();
        if !t.is_empty() {
            parts.push(t.to_string());
        }
    }
    if let Some(ref authors) = entry.authors {
        let a = authors.trim();
        if !a.is_empty() {
            parts.push(a.to_string());
        }
    }
    if let Some(ref venue) = entry.venue {
        let v = venue.trim();
        if !v.is_empty() {
            parts.push(v.to_string());
        }
    }
    if let Some(year) = entry.year {
        parts.push(year.to_string());
    }

    if parts.is_empty() {
        entry.raw_text.trim().to_string()
    } else {
        parts.join(" ")
    }
}

/// Compute 16-character hex hash of paper draft text for staleness detection.
pub fn compute_draft_hash(text: &str) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
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
        let status = probe_source(Utf8Path::new("/no/such/file.pdf")).unwrap();
        assert_eq!(status, DocumentStatus::NotFound);
    }

    #[test]
    fn validate_valid_pdf_magic() {
        let mut f = NamedTempFile::new().unwrap();
        f.write_all(b"%PDF-1.4\n%sil-test\n").unwrap();
        let path = Utf8PathBuf::from_path_buf(f.path().to_path_buf()).unwrap();
        assert_eq!(
            probe_source(&path).unwrap(),
            DocumentStatus::Valid(SourceKind::Pdf)
        );
        assert_eq!(validate_pdf_path(&path).unwrap(), DocumentStatus::ValidPdf);
    }

    #[test]
    fn probe_source_markdown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("notes.md");
        std::fs::write(&path, b"# Title\nSome markdown text.").unwrap();
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        assert_eq!(
            probe_source(&path).unwrap(),
            DocumentStatus::Valid(SourceKind::Markdown)
        );
    }

    #[test]
    fn probe_source_text() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.txt");
        std::fs::write(&path, b"Plain text content").unwrap();
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        assert_eq!(
            probe_source(&path).unwrap(),
            DocumentStatus::Valid(SourceKind::Text)
        );
    }

    #[test]
    fn probe_source_html() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("paper.html");
        std::fs::write(&path, b"<html><body>Hello</body></html>").unwrap();
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        assert_eq!(
            probe_source(&path).unwrap(),
            DocumentStatus::Valid(SourceKind::Html)
        );
    }

    #[test]
    fn probe_source_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("data.unknown");
        std::fs::write(&path, b"random bytes").unwrap();
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        assert_eq!(
            probe_source(&path).unwrap(),
            DocumentStatus::UnsupportedFormat
        );
    }

    #[test]
    fn document_status_parseable() {
        assert!(DocumentStatus::Valid(SourceKind::Pdf).is_parseable());
        assert!(DocumentStatus::Valid(SourceKind::Markdown).is_parseable());
        assert!(!DocumentStatus::AlreadyParsed.is_parseable());
        assert!(!DocumentStatus::NotFound.is_parseable());
        assert!(!DocumentStatus::NotPdf.is_parseable());
        assert!(!DocumentStatus::Corrupted.is_parseable());
    }

    #[test]
    fn document_status_messages() {
        for s in [
            DocumentStatus::Valid(SourceKind::Pdf),
            DocumentStatus::Valid(SourceKind::Markdown),
            DocumentStatus::NotFound,
            DocumentStatus::NotPdf,
            DocumentStatus::UnsupportedFormat,
            DocumentStatus::AlreadyParsed,
            DocumentStatus::Corrupted,
        ] {
            assert!(!s.message().is_empty());
            assert_eq!(s.to_string(), s.message());
        }
    }

    #[test]
    fn source_document_new_sets_filename_and_kind() {
        let doc = SourceDocument::new("sources/foo.pdf".into());
        assert_eq!(doc.filename, "foo.pdf");
        assert_eq!(doc.kind, SourceKind::Pdf);
        assert!(!doc.parsed);
        assert!(doc.status.is_none());
        assert!(doc.authors.is_none());
        assert!(doc.abstract_text.is_none());
        assert!(doc.doi.is_none());
        assert!(doc.year.is_none());
        assert!(doc.venue.is_none());

        let doc_md = SourceDocument::new("sources/notes.md".into());
        assert_eq!(doc_md.kind, SourceKind::Markdown);
    }

    #[test]
    fn source_id_from_str_and_string() {
        let a: SourceId = "a.pdf".into();
        let b = SourceId::from("b.pdf".to_string());
        assert_eq!(a.as_str(), "a.pdf");
        assert_eq!(b.as_str(), "b.pdf");
    }

    #[test]
    fn validate_corrupted_pdf_extension() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.pdf");
        std::fs::write(&path, b"%PD").unwrap();
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        let status = probe_source(&path).unwrap();
        assert_eq!(status, DocumentStatus::Corrupted);
    }

    #[test]
    fn validate_directory_is_not_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let status = probe_source(&path).unwrap();
        assert_eq!(status, DocumentStatus::NotPdf);
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

    #[test]
    fn test_strip_latex_for_embed() {
        let latex = "% Comment line\n\\section{Introduction}\nWe present a novel method \\cite{paper2024} for \\textbf{machine learning}.\n";
        let stripped = strip_latex_for_embed(latex);
        assert!(!stripped.contains("Comment line"));
        assert!(!stripped.contains("\\section"));
        assert!(stripped.contains("Introduction"));
        assert!(stripped.contains("We present a novel method"));
        assert!(stripped.contains("machine learning"));
    }

    #[test]
    fn test_ref_text_for_embed() {
        let entry = ReferenceEntry {
            id: "ref-1".into(),
            source_id: SourceId::from_sources_relative(camino::Utf8Path::new("test.pdf")),
            ref_index: 1,
            raw_text: "Unused raw text".into(),
            title: Some("Attention Is All You Need".into()),
            authors: Some("Vaswani et al.".into()),
            year: Some(2017),
            venue: Some("NeurIPS".into()),
            doi: None,
            arxiv_id: None,
            url: None,
        };
        let text = ref_text_for_embed(&entry);
        assert_eq!(text, "Attention Is All You Need Vaswani et al. NeurIPS 2017");

        let empty_entry = ReferenceEntry {
            id: "ref-2".into(),
            source_id: SourceId::from_sources_relative(camino::Utf8Path::new("test.pdf")),
            ref_index: 2,
            raw_text: "Full raw citation string".into(),
            title: None,
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };
        assert_eq!(ref_text_for_embed(&empty_entry), "Full raw citation string");
    }

    #[test]
    fn test_should_attempt_metadata_fetch() {
        let empty_entry = ReferenceEntry {
            id: "ref_1".into(),
            source_id: "src_1".into(),
            ref_index: 1,
            raw_text: "some raw ref".into(),
            title: None,
            authors: None,
            year: None,
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };
        assert!(!should_attempt_metadata_fetch(&empty_entry));
        assert!(!empty_entry.should_attempt_metadata_fetch());

        let doi_entry = ReferenceEntry {
            doi: Some("10.1000/182".into()),
            ..empty_entry.clone()
        };
        assert!(should_attempt_metadata_fetch(&doi_entry));

        let arxiv_entry = ReferenceEntry {
            arxiv_id: Some("2405.12345".into()),
            ..empty_entry.clone()
        };
        assert!(should_attempt_metadata_fetch(&arxiv_entry));

        let title_entry = ReferenceEntry {
            title: Some("Attention Is All You Need".into()),
            ..empty_entry.clone()
        };
        assert!(should_attempt_metadata_fetch(&title_entry));

        let blank_title_entry = ReferenceEntry {
            title: Some("   ".into()),
            ..empty_entry.clone()
        };
        assert!(!should_attempt_metadata_fetch(&blank_title_entry));
    }
}
