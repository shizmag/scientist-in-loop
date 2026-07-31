//! Source fetching module.

use std::path::PathBuf;
use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};

use crate::error::ParseError;

/// Fetch paper/source by DOI, arXiv ID, or URL into destination directory.
pub fn fetch_source_target(
    target: &str,
    destination_dir: &Utf8Path,
) -> Result<Utf8PathBuf, ParseError> {
    std::fs::create_dir_all(destination_dir.as_str()).map_err(|e| {
        ParseError::Message(format!(
            "failed to create destination directory '{destination_dir}': {e}"
        ))
    })?;

    let script = discover_download_script()?;
    let python = std::env::var("SIL_PYTHON").unwrap_or_else(|_| "python3".into());

    let output = Command::new(&python)
        .arg(script.as_str())
        .arg(target)
        .arg(destination_dir.as_str())
        .output()
        .map_err(|e| ParseError::Message(format!("failed to spawn {python} {script}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(ParseError::Message(format!(
            "download failed: {}\n{}",
            stderr.trim(),
            stdout.trim()
        )));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let saved = stdout
        .lines()
        .rev()
        .find(|l| l.trim().ends_with(".pdf") || l.contains("sources/"))
        .map(|l| l.trim().to_string())
        .unwrap_or_else(|| stdout.trim().to_string());

    Ok(Utf8PathBuf::from(saved))
}

fn discover_download_script() -> Result<Utf8PathBuf, ParseError> {
    if let Ok(p) = std::env::var("SIL_DOWNLOAD_SCRIPT") {
        return Ok(Utf8PathBuf::from(p));
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(manifest) = std::env::var("CARGO_MANIFEST_DIR") {
        let m = PathBuf::from(manifest);
        candidates.push(m.join("../../python/download_pdf.py"));
        candidates.push(m.join("../python/download_pdf.py"));
        candidates.push(m.join("python/download_pdf.py"));
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("python/download_pdf.py"));
        candidates.push(cwd.join("../python/download_pdf.py"));
    }
    if let Ok(exe) = std::env::current_exe()
        && let Some(dir) = exe.parent()
    {
        candidates.push(dir.join("python/download_pdf.py"));
        candidates.push(dir.join("../python/download_pdf.py"));
        candidates.push(dir.join("../../python/download_pdf.py"));
    }
    for c in candidates {
        if c.is_file() {
            return Utf8PathBuf::from_path_buf(c)
                .map_err(|_| ParseError::Message("download script path not utf-8".into()));
        }
    }
    Err(ParseError::Message(
        "could not locate python/download_pdf.py; set SIL_DOWNLOAD_SCRIPT".into(),
    ))
}
