//! PDF parsing orchestration via Marker (Python helper).
//!
//! Stage 0: skeleton with validation + pluggable runner.
//! Stage 3: full parse pipeline + interactive selection.

#![deny(missing_docs)]

use std::path::{Path, PathBuf};
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{
    DocumentStatus, SilError, SilUi, SourceDocument, SourceId, validate_pdf_path,
};
use sil_db::SilDb;
use thiserror::Error;

/// Parse-related errors.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Document validation failed.
    #[error("{0}")]
    InvalidDocument(String),
    /// Python / Marker invocation failed.
    #[error("Marker parse failed: {0}")]
    Marker(String),
    /// Database write failed.
    #[error("database: {0}")]
    Db(String),
    /// Other.
    #[error("{0}")]
    Message(String),
}

impl From<ParseError> for SilError {
    fn from(value: ParseError) -> Self {
        SilError::Parse(value.to_string())
    }
}

/// Result of parsing one PDF.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Source document metadata.
    pub document: SourceDocument,
    /// Extracted plain text / markdown.
    pub content: String,
}

/// Abstraction over the Marker Python helper (for tests).
pub trait MarkerRunner: Send + Sync {
    /// Parse `pdf` and return extracted text.
    fn parse_pdf(&self, pdf: &Utf8Path) -> Result<String, ParseError>;
}

/// Real runner invoking `python/parse_with_marker.py`.
#[derive(Debug, Clone)]
pub struct PythonMarkerRunner {
    /// Path to the Python script.
    pub script: Utf8PathBuf,
    /// Python executable.
    pub python: String,
}

impl PythonMarkerRunner {
    /// Create runner with script path and default `python3`.
    pub fn new(script: impl Into<Utf8PathBuf>) -> Self {
        Self {
            script: script.into(),
            python: std::env::var("SIL_PYTHON").unwrap_or_else(|_| "python3".into()),
        }
    }

    /// Locate the script relative to the sil install / workspace.
    pub fn discover() -> Result<Self, ParseError> {
        if let Ok(p) = std::env::var("SIL_PARSE_SCRIPT") {
            return Ok(Self::new(Utf8PathBuf::from(p)));
        }
        // Walk from CARGO_MANIFEST_DIR-style candidates and cwd.
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            // crates/sil or crates/sil-parse → workspace root
            let m = PathBuf::from(manifest);
            candidates.push(m.join("../../python/parse_with_marker.py"));
            candidates.push(m.join("../python/parse_with_marker.py"));
            candidates.push(m.join("python/parse_with_marker.py"));
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("python/parse_with_marker.py"));
            candidates.push(cwd.join("../python/parse_with_marker.py"));
        }
        // Relative to executable.
        if let Ok(exe) = std::env::current_exe()
            && let Some(dir) = exe.parent()
        {
            candidates.push(dir.join("python/parse_with_marker.py"));
            candidates.push(dir.join("../python/parse_with_marker.py"));
            candidates.push(dir.join("../../python/parse_with_marker.py"));
        }
        for c in candidates {
            if c.is_file() {
                let utf = Utf8PathBuf::from_path_buf(c).map_err(|_| {
                    ParseError::Message("parse script path is not UTF-8".into())
                })?;
                return Ok(Self::new(utf));
            }
        }
        Err(ParseError::Message(
            "could not locate python/parse_with_marker.py; set SIL_PARSE_SCRIPT".into(),
        ))
    }
}

impl MarkerRunner for PythonMarkerRunner {
    fn parse_pdf(&self, pdf: &Utf8Path) -> Result<String, ParseError> {
        let output = Command::new(&self.python)
            .arg(self.script.as_str())
            .arg(pdf.as_str())
            .output()
            .map_err(|e| {
                ParseError::Marker(format!(
                    "failed to spawn {} {}: {e}",
                    self.python, self.script
                ))
            })?;
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            let stdout = String::from_utf8_lossy(&output.stdout);
            return Err(ParseError::Marker(format!(
                "exit {}: {}\n{}",
                output.status.code().unwrap_or(-1),
                stderr.trim(),
                stdout.trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    }
}

/// Stub runner that returns fixed text (tests).
#[derive(Debug, Clone)]
pub struct StubMarkerRunner {
    /// Content to return for any PDF.
    pub content: String,
}

impl MarkerRunner for StubMarkerRunner {
    fn parse_pdf(&self, pdf: &Utf8Path) -> Result<String, ParseError> {
        Ok(format!(
            "# Parsed from {}\n\n{}",
            pdf.file_name().unwrap_or("unknown"),
            self.content
        ))
    }
}

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
    if matches!(status, DocumentStatus::ValidPdf) && db.is_parsed(&id).unwrap_or(false) {
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
    // Derive a simple title from first markdown heading if present.
    doc.title = content.lines().find_map(|l| {
        let t = l.trim();
        t.strip_prefix("# ").map(str::to_string)
    });
    doc.parsed = true;
    doc.status = Some(DocumentStatus::ValidPdf);

    db.upsert_parsed(&doc, &content)
        .map_err(|e| ParseError::Db(e.to_string()))?;

    Ok(ParseResult {
        document: doc,
        content,
    })
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
        let utf = Utf8PathBuf::from_path_buf(path).map_err(|_| {
            ParseError::Message("non-UTF8 path in sources/".into())
        })?;
        let filename = utf.file_name().unwrap_or("").to_string();
        let id = SourceId::new(filename);
        if !db.is_parsed(&id).unwrap_or(false) {
            paths.push(utf);
        }
    }
    paths.sort();
    Ok(paths)
}

/// Interactive multi-select over paths. Returns selected indices.
/// Non-interactive / empty: returns all indices (parse all unparsed).
pub fn select_pdfs_interactive(
    paths: &[Utf8PathBuf],
    ui: &dyn SilUi,
) -> Result<Vec<usize>, ParseError> {
    if paths.is_empty() {
        ui.warn("No unparsed PDFs found in sources/.");
        return Ok(Vec::new());
    }
    if !ui.interactive() {
        // Non-interactive: select all.
        ui.info(&format!(
            "Non-interactive mode: selecting all {} unparsed PDF(s).",
            paths.len()
        ));
        return Ok((0..paths.len()).collect());
    }

    // Simple keyboard-driven toggle UI via stdin lines.
    // Commands: numbers toggle, a=all, n=none, Enter=confirm, q=quit
    use std::io::{self, BufRead, Write};
    let mut selected: Vec<bool> = vec![true; paths.len()];
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        ui.println("");
        ui.info("Select PDFs to parse (toggle by number, a=all, n=none, Enter=confirm, q=cancel):");
        for (i, p) in paths.iter().enumerate() {
            let mark = if selected[i] { "[x]" } else { "[ ]" };
            ui.println(&format!(
                "  {:>2}. {} {}",
                i + 1,
                mark,
                p.file_name().unwrap_or(p.as_str())
            ));
        }
        write!(stdout, "> ").ok();
        stdout.flush().ok();
        let mut line = String::new();
        if stdin.lock().read_line(&mut line).is_err() {
            break;
        }
        let line = line.trim();
        if line.is_empty() {
            break;
        }
        if line.eq_ignore_ascii_case("q") {
            return Ok(Vec::new());
        }
        if line.eq_ignore_ascii_case("a") {
            selected.fill(true);
            continue;
        }
        if line.eq_ignore_ascii_case("n") {
            selected.fill(false);
            continue;
        }
        for token in line.split_whitespace() {
            if let Ok(n) = token.parse::<usize>()
                && (1..=paths.len()).contains(&n)
            {
                selected[n - 1] = !selected[n - 1];
            }
        }
    }

    Ok(selected
        .iter()
        .enumerate()
        .filter_map(|(i, s)| if *s { Some(i) } else { None })
        .collect())
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

/// Minimal valid PDF bytes for fixtures.
pub fn minimal_pdf_bytes() -> &'static [u8] {
    // Tiny but valid-enough PDF for magic checks; Marker may fail on it.
    b"%PDF-1.1\n%\xe2\xe3\xcf\xd3\n1 0 obj<<>>endobj\ntrailer<<>>\n%%EOF\n"
}

/// Write a fixture PDF to `path`.
pub fn write_fixture_pdf(path: &Path) -> std::io::Result<()> {
    std::fs::write(path, minimal_pdf_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::NullUi;
    use sil_db::SilDb;

    #[test]
    fn reject_missing() {
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "hello".into(),
        };
        let err = parse_one(Utf8Path::new("/no/such.pdf"), &db, &runner, &ui).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn parse_with_stub() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("paper.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let path = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "transformer attention mechanism".into(),
        };
        let result = parse_one(&path, &db, &runner, &ui).unwrap();
        assert!(result.document.parsed);
        assert!(db.is_parsed(&result.document.id).unwrap());
        // Second parse → already parsed
        let err = parse_one(&path, &db, &runner, &ui).unwrap_err();
        assert!(err.to_string().contains("already parsed"));
    }

    #[test]
    fn reject_non_pdf() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("notes.txt");
        std::fs::write(&f, "not a pdf").unwrap();
        let path = Utf8PathBuf::from_path_buf(f).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "x".into(),
        };
        let err = parse_one(&path, &db, &runner, &ui).unwrap_err();
        assert!(err.to_string().to_lowercase().contains("not a pdf"));
    }

    #[test]
    fn noninteractive_select_all() {
        let paths = vec![
            Utf8PathBuf::from("a.pdf"),
            Utf8PathBuf::from("b.pdf"),
        ];
        let ui = NullUi::new();
        let sel = select_pdfs_interactive(&paths, &ui).unwrap();
        assert_eq!(sel, vec![0, 1]);
    }
}
