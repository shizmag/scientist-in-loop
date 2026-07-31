//! Marker runner abstraction (CLI & Python helper).

use std::path::PathBuf;
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use tempfile::tempdir;

use crate::error::ParseError;

/// Abstraction over the Marker runner (CLI tool or Python helper).
pub trait MarkerRunner: Send + Sync {
    /// Parse `pdf` and return extracted text.
    fn parse_pdf(&self, pdf: &Utf8Path) -> Result<String, ParseError>;
}

/// Helper function to search for executable binary in system PATH or direct path.
fn find_binary_in_path(binary_name: &str) -> Option<Utf8PathBuf> {
    if binary_name.contains('/') || binary_name.contains('\\') {
        let p = PathBuf::from(binary_name);
        if p.is_file() {
            return Utf8PathBuf::from_path_buf(p).ok();
        }
        return None;
    }
    if let Ok(path_var) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path_var) {
            let candidate = dir.join(binary_name);
            if candidate.is_file()
                && let Ok(utf) = Utf8PathBuf::from_path_buf(candidate) {
                    return Some(utf);
                }
        }
    }
    None
}

/// Runner invoking pre-installed `marker_single` (or `marker`) CLI binary using isolated temp files.
#[derive(Debug, Clone)]
pub struct CliMarkerRunner {
    /// Path to the marker CLI binary (e.g. `marker_single`).
    pub binary: Utf8PathBuf,
}

impl CliMarkerRunner {
    /// Create runner with explicit binary path.
    pub fn new(binary: impl Into<Utf8PathBuf>) -> Self {
        Self {
            binary: binary.into(),
        }
    }

    /// Discover pre-installed marker CLI utility (`SIL_MARKER_BIN`, `marker_single`, or `marker`).
    pub fn discover() -> Result<Self, ParseError> {
        if let Ok(p) = std::env::var("SIL_MARKER_BIN") {
            return Ok(Self::new(Utf8PathBuf::from(p)));
        }
        for name in &["marker_single", "marker"] {
            if let Some(path) = find_binary_in_path(name) {
                return Ok(Self::new(path));
            }
        }
        Err(ParseError::Message(
            "could not locate marker_single or marker CLI binary in PATH; set SIL_MARKER_BIN".into(),
        ))
    }
}

impl MarkerRunner for CliMarkerRunner {
    fn parse_pdf(&self, pdf: &Utf8Path) -> Result<String, ParseError> {
        let tmp = tempdir().map_err(|e| ParseError::Marker(format!("failed to create temp dir: {e}")))?;
        let mut cmd = Command::new(&self.binary);
        cmd.arg(pdf.as_str())
            .arg("--output_dir")
            .arg(tmp.path())
            .arg("--output_format")
            .arg("markdown")
            .arg("--disable_image_extraction");

        if let Ok(flags) = std::env::var("SIL_MARKER_FLAGS") {
            for flag in flags.split_whitespace() {
                cmd.arg(flag);
            }
        }

        let output = cmd.output().map_err(|e| {
            ParseError::Marker(format!(
                "failed to spawn {}: {e}",
                self.binary
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

        fn find_md_file(dir: &std::path::Path) -> Option<PathBuf> {
            if let Ok(entries) = std::fs::read_dir(dir) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md") {
                        return Some(path);
                    } else if path.is_dir()
                        && let Some(found) = find_md_file(&path) {
                            return Some(found);
                        }
                }
            }
            None
        }

        let md_file = find_md_file(tmp.path()).ok_or_else(|| {
            ParseError::Marker(format!(
                "marker CLI executed successfully but produced no markdown output in {}",
                tmp.path().display()
            ))
        })?;

        let content = std::fs::read_to_string(&md_file).map_err(|e| {
            ParseError::Marker(format!(
                "failed to read extracted markdown file {}: {e}",
                md_file.display()
            ))
        })?;

        Ok(content)
    }
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
        let mut candidates: Vec<PathBuf> = Vec::new();
        if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
            let m = PathBuf::from(manifest);
            candidates.push(m.join("../../python/parse_with_marker.py"));
            candidates.push(m.join("../python/parse_with_marker.py"));
            candidates.push(m.join("python/parse_with_marker.py"));
        }
        if let Ok(cwd) = std::env::current_dir() {
            candidates.push(cwd.join("python/parse_with_marker.py"));
            candidates.push(cwd.join("../python/parse_with_marker.py"));
        }
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

/// Discover best available Marker runner based on environment and system setup.
/// Priority:
/// 1. `SIL_MARKER_STUB` -> `StubMarkerRunner`
/// 2. `SIL_MARKER_BIN` / `marker_single` / `marker` in PATH -> `CliMarkerRunner`
/// 3. `SIL_PARSE_SCRIPT` / `python/parse_with_marker.py` -> `PythonMarkerRunner`
pub fn discover_marker_runner() -> Result<Box<dyn MarkerRunner>, ParseError> {
    if let Ok(stub) = std::env::var("SIL_MARKER_STUB") {
        return Ok(Box::new(StubMarkerRunner { content: stub }));
    }
    if let Ok(cli) = CliMarkerRunner::discover() {
        return Ok(Box::new(cli));
    }
    if let Ok(py) = PythonMarkerRunner::discover() {
        return Ok(Box::new(py));
    }
    Err(ParseError::Message(
        "could not locate marker_single CLI binary (pip install marker-pdf) or python/parse_with_marker.py helper script; set SIL_MARKER_BIN or SIL_PARSE_SCRIPT".into(),
    ))
}

