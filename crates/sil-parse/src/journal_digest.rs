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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_missing_script_returns_empty() {
        let missing = Utf8Path::new("/nonexistent/fetch_script.py");
        let res = fetch_journal_publications("quantum", 5, Some(missing), None).unwrap();
        assert!(res.is_empty());
    }

    #[test]
    fn test_mock_python_script_success() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("mock_digest.py");
        std::fs::write(
            &script_path,
            r#"
import json
print(json.dumps([
  {
    "doi": "10.1038/s41586-023-00000-0",
    "title": "Quantum Supremacy",
    "authors": "A. Scientist",
    "journal": "Nature",
    "year": 2024,
    "abstract_text": "Sample abstract",
    "citation_count": 100,
    "url": "https://doi.org/10.1038/s41586-023-00000-0",
    "pdf_url": None
  }
]))
"#,
        )
        .unwrap();

        let path = Utf8PathBuf::from_path_buf(script_path).unwrap();
        let items = fetch_journal_publications("quantum", 5, Some(&path), Some("python3")).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].title, "Quantum Supremacy");
        assert_eq!(items[0].journal, "Nature");
    }

    #[test]
    fn test_mock_python_script_failure_returns_err() {
        let dir = tempfile::tempdir().unwrap();
        let script_path = dir.path().join("failing_digest.py");
        std::fs::write(&script_path, "import sys; sys.stderr.write('API Error'); sys.exit(1)").unwrap();

        let path = Utf8PathBuf::from_path_buf(script_path).unwrap();
        let err = fetch_journal_publications("quantum", 5, Some(&path), Some("python3")).unwrap_err();
        assert!(err.to_string().contains("API Error"));
    }
}

