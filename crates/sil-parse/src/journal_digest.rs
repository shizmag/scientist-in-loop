//! Top-journal publication digest runner calling Python script `fetch_journal_digest.py`.

use std::process::Command;
use camino::{Utf8Path, Utf8PathBuf};
use sil_core::JournalPublication;
use crate::error::ParseError;

/// Fetch top journal publications matching a query using `fetch_journal_digest.py`.
pub fn fetch_journal_publications(
    query: &str,
    limit: usize,
    script_path: Option<&Utf8Path>,
    python_bin: Option<&str>,
) -> Result<Vec<JournalPublication>, ParseError> {
    let python = python_bin.unwrap_or("python3");
    let script = script_path
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| Utf8PathBuf::from("python/fetch_journal_digest.py"));

    if !script.exists() {
        // Return empty or fallback cleanly if script is missing
        return Ok(Vec::new());
    }

    let output = Command::new(python)
        .arg(script.as_str())
        .arg(query)
        .arg(limit.to_string())
        .output()
        .map_err(|e| ParseError::Marker(format!("Failed to execute {python} {script}: {e}")))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(ParseError::Marker(format!("fetch_journal_digest.py failed: {stderr}")));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let items: Vec<JournalPublication> = serde_json::from_str(&stdout).map_err(|e| {
        ParseError::Marker(format!("Failed to parse journal digest JSON output: {e}"))
    })?;

    Ok(items)

}
