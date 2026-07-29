//! Marker Python helper invocation.

use std::path::PathBuf;
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};

use crate::error::ParseError;

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
