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
                && let Ok(utf) = Utf8PathBuf::from_path_buf(candidate)
            {
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
            "could not locate marker_single or marker CLI binary in PATH; set SIL_MARKER_BIN"
                .into(),
        ))
    }
}

fn find_gs_binary() -> Option<Utf8PathBuf> {
    if let Some(path) = find_binary_in_path("gs") {
        return Some(path);
    }
    for candidate in &["/opt/homebrew/bin/gs", "/usr/local/bin/gs", "/usr/bin/gs"] {
        let p = PathBuf::from(candidate);
        if p.is_file()
            && let Ok(utf) = Utf8PathBuf::from_path_buf(p) {
                return Some(utf);
            }
    }
    None
}

fn repair_pdf_with_gs(gs_bin: &Utf8Path, input_pdf: &Utf8Path, output_pdf: &Utf8Path) -> bool {
    let status = Command::new(gs_bin)
        .arg("-o")
        .arg(output_pdf.as_str())
        .arg("-sDEVICE=pdfwrite")
        .arg(input_pdf.as_str())
        .output();
    if let Ok(out) = status {
        out.status.success() && output_pdf.is_file()
    } else {
        false
    }
}

impl MarkerRunner for CliMarkerRunner {
    fn parse_pdf(&self, pdf: &Utf8Path) -> Result<String, ParseError> {
        let run_marker = |target: &Utf8Path| -> Result<String, ParseError> {
            let tmp = tempdir()
                .map_err(|e| ParseError::Marker(format!("failed to create temp dir: {e}")))?;
            let mut cmd = Command::new(&self.binary);
            cmd.arg(target.as_str())
                .arg("--output_dir")
                .arg(tmp.path())
                .arg("--output_format")
                .arg("markdown")
                .arg("--disable_image_extraction")
                .arg("--disable_multiprocessing");

            if let Ok(flags) = std::env::var("SIL_MARKER_FLAGS") {
                for flag in flags.split_whitespace() {
                    cmd.arg(flag);
                }
            }

            let output = cmd
                .output()
                .map_err(|e| ParseError::Marker(format!("failed to spawn {}: {e}", self.binary)))?;

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
                        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("md")
                        {
                            return Some(path);
                        } else if path.is_dir()
                            && let Some(found) = find_md_file(&path)
                        {
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
        };

        match run_marker(pdf) {
            Ok(content) => Ok(content),
            Err(first_err) => {
                if let Some(gs_bin) = find_gs_binary()
                    && let Ok(repair_dir) = tempdir()
                    && let Ok(repaired_path) =
                        Utf8PathBuf::from_path_buf(repair_dir.path().join("repaired.pdf"))
                    && repair_pdf_with_gs(&gs_bin, pdf, &repaired_path)
                        && let Ok(repaired_content) = run_marker(&repaired_path) {
                            return Ok(repaired_content);
                        }
                Err(first_err)
            }
        }
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
                let utf = Utf8PathBuf::from_path_buf(c)
                    .map_err(|_| ParseError::Message("parse script path is not UTF-8".into()))?;
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
    if std::env::var("SIL_PARSE_SCRIPT").is_ok()
        && let Ok(py) = PythonMarkerRunner::discover() {
            return Ok(Box::new(py));
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn test_find_binary_in_path_direct_file() {
        let dir = tempdir().unwrap();
        let bin_path = dir.path().join("mock_binary");
        fs::write(&bin_path, "#!/bin/sh\nexit 0").unwrap();

        let utf = Utf8PathBuf::from_path_buf(bin_path.clone()).unwrap();
        assert_eq!(find_binary_in_path(utf.as_str()), Some(utf));

        assert_eq!(find_binary_in_path("/nonexistent/bin/path"), None);
    }

    #[test]
    fn test_cli_marker_runner_discover_env() {
        let dir = tempdir().unwrap();
        let bin_path = dir.path().join("custom_marker_cli");
        fs::write(&bin_path, "#!/bin/sh\n").unwrap();

        unsafe {
            std::env::set_var("SIL_MARKER_BIN", bin_path.to_str().unwrap());
        }

        let runner = CliMarkerRunner::discover().unwrap();
        assert_eq!(runner.binary.as_str(), bin_path.to_str().unwrap());

        unsafe {
            std::env::remove_var("SIL_MARKER_BIN");
        }
    }

    #[test]
    fn test_cli_marker_runner_parse_pdf_mock() {
        let dir = tempdir().unwrap();
        let bin_path = dir.path().join("mock_cli_marker.sh");
        fs::write(
            &bin_path,
            "#!/bin/sh\nOUT_DIR=\"\"\nfor arg in \"$@\"; do\n  if [ \"$PREV\" = \"--output_dir\" ]; then OUT_DIR=\"$arg\"; fi\n  PREV=\"$arg\"\ndone\nmkdir -p \"$OUT_DIR\"\necho '# Mock Output' > \"$OUT_DIR/out.md\"\nexit 0\n",
        )
        .unwrap();

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&bin_path).unwrap().permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&bin_path, perms).unwrap();
        }

        let runner = CliMarkerRunner::new(Utf8PathBuf::from_path_buf(bin_path).unwrap());
        let dummy_pdf = Utf8PathBuf::from_path_buf(dir.path().join("test.pdf")).unwrap();
        fs::write(dummy_pdf.as_str(), "%PDF-1.4 dummy").unwrap();

        unsafe {
            std::env::set_var("SIL_MARKER_FLAGS", "--debug");
        }

        let content = runner.parse_pdf(&dummy_pdf).unwrap();

        unsafe {
            std::env::remove_var("SIL_MARKER_FLAGS");
        }

        assert!(content.contains("# Mock Output"));
    }

    #[test]
    fn test_python_marker_runner_discover_env() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("parse_with_marker.py");
        fs::write(&script_path, "print('ok')").unwrap();

        unsafe {
            std::env::set_var("SIL_PARSE_SCRIPT", script_path.to_str().unwrap());
        }

        let runner = PythonMarkerRunner::discover().unwrap();
        assert_eq!(runner.script.as_str(), script_path.to_str().unwrap());

        unsafe {
            std::env::remove_var("SIL_PARSE_SCRIPT");
        }
    }

    #[test]
    fn test_discover_marker_runner_stub_priority() {
        unsafe {
            std::env::set_var("SIL_MARKER_STUB", "Test Stub Content");
        }

        let runner = discover_marker_runner().unwrap();
        let dummy_pdf = Utf8Path::new("dummy.pdf");
        let parsed = runner.parse_pdf(dummy_pdf).unwrap();

        unsafe {
            std::env::remove_var("SIL_MARKER_STUB");
        }

        assert!(parsed.contains("Test Stub Content"));
        assert!(parsed.contains("dummy.pdf"));
    }

    #[test]
    fn test_discover_marker_runner_parse_script_priority() {
        let dir = tempdir().unwrap();
        let script_path = dir.path().join("parse_with_marker.py");
        fs::write(&script_path, "print('ok')").unwrap();

        unsafe {
            std::env::set_var("SIL_PARSE_SCRIPT", script_path.to_str().unwrap());
        }

        let runner = discover_marker_runner();

        unsafe {
            std::env::remove_var("SIL_PARSE_SCRIPT");
        }

        assert!(runner.is_ok());
    }
}

