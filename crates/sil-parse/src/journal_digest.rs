//! Top-journal publication digest feed and Crossref metadata hydration natively in Rust.

use crate::error::ParseError;
use camino::{Utf8Path, Utf8PathBuf};
use sil_core::JournalPublication;
use std::process::Command;

fn format_authors(value: &serde_json::Value) -> String {
    let mut names = Vec::new();
    if let Some(arr) = value.as_array() {
        for obj in arr {
            if let Some(name) = obj.get("name").and_then(|v| v.as_str()) {
                names.push(name.to_string());
            } else {
                let given = obj.get("given").and_then(|v| v.as_str()).unwrap_or("");
                let family = obj.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let full = match (given.is_empty(), family.is_empty()) {
                    (false, false) => format!("{given} {family}"),
                    (false, true) => given.to_string(),
                    (true, false) => family.to_string(),
                    (true, true) => String::new(),
                };
                if !full.is_empty() {
                    names.push(full);
                }
            }
        }
    }
    names.join(", ")
}

fn extract_year_from_crossref(item: &serde_json::Value) -> Option<u32> {
    for key in [
        "published-print",
        "published-online",
        "published",
        "issued",
        "created",
    ] {
        if let Some(dp) = item
            .get(key)
            .and_then(|v| v.get("date-parts"))
            .and_then(|v| v.as_array())
            && let Some(first_date) = dp.first().and_then(|v| v.as_array())
            && let Some(year_val) = first_date.first().and_then(|v| v.as_u64())
            && (1800..=2030).contains(&year_val)
        {
            return Some(year_val as u32);
        }
    }
    None
}

fn clean_abstract(raw: &str) -> String {
    let mut result = String::new();
    let mut in_tag = false;
    for c in raw.chars() {
        if c == '<' {
            in_tag = true;
        } else if c == '>' {
            in_tag = false;
        } else if !in_tag {
            result.push(c);
        }
    }
    result.trim().to_string()
}

fn extract_pdf_url(item: &serde_json::Value) -> Option<String> {
    if let Some(links) = item.get("link").and_then(|v| v.as_array()) {
        for link in links {
            let ct = link
                .get("content-type")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let url = link.get("URL").and_then(|v| v.as_str()).unwrap_or("");
            if (ct.contains("pdf") || url.ends_with(".pdf")) && !url.is_empty() {
                return Some(url.to_string());
            }
        }
        if let Some(first_url) = links
            .first()
            .and_then(|l| l.get("URL"))
            .and_then(|v| v.as_str())
            && !first_url.is_empty()
        {
            return Some(first_url.to_string());
        }
    }
    None
}

/// Convert a Crossref JSON work item into a domain `JournalPublication`.
pub fn parse_crossref_item(item: &serde_json::Value) -> Option<JournalPublication> {
    let doi = item
        .get("DOI")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    let title = item
        .get("title")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    if title.is_empty() && doi.is_none() {
        return None;
    }

    let authors = format_authors(item.get("author").unwrap_or(&serde_json::Value::Null));

    let journal = item
        .get("container-title")
        .and_then(|v| v.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    let year = extract_year_from_crossref(item);

    let abstract_text = item
        .get("abstract")
        .and_then(|v| v.as_str())
        .map(clean_abstract)
        .unwrap_or_default();

    let citation_count = item
        .get("is-referenced-by-count")
        .and_then(|v| v.as_u64())
        .map(|c| c as u32);

    let url = item
        .get("URL")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            doi.as_ref()
                .map(|d| format!("https://doi.org/{d}"))
                .unwrap_or_default()
        });

    let pdf_url = extract_pdf_url(item);

    Some(JournalPublication {
        doi,
        title,
        authors,
        journal,
        year,
        abstract_text,
        citation_count,
        url,
        pdf_url,
    })
}

/// Fetch publications directly from Crossref API natively in Rust using `ureq`.
pub fn fetch_journal_publications_native(
    query: &str,
    limit: usize,
) -> Result<Vec<JournalPublication>, ParseError> {
    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    let response = agent
        .get("https://api.crossref.org/works")
        .set("User-Agent", "scientist-in-loop/0.1.0")
        .query("query", query)
        .query("rows", &limit.to_string())
        .call()
        .map_err(|e| ParseError::Message(format!("Crossref API request failed: {e}")))?;

    let json: serde_json::Value = response
        .into_json()
        .map_err(|e| ParseError::Message(format!("Failed to parse Crossref response JSON: {e}")))?;

    let items = json
        .get("message")
        .and_then(|m| m.get("items"))
        .and_then(|i| i.as_array())
        .ok_or_else(|| ParseError::Message("Invalid Crossref API payload structure".to_string()))?;

    let mut publications = Vec::new();
    for item in items {
        if let Some(pub_item) = parse_crossref_item(item) {
            publications.push(pub_item);
        }
    }

    Ok(publications)
}

/// Fetch single paper's metadata from Crossref API natively in Rust using `ureq`.
pub fn fetch_work_by_doi(doi: &str) -> Result<Option<JournalPublication>, ParseError> {
    let clean_doi = doi.trim_start_matches("doi:").trim();
    let url = format!("https://api.crossref.org/works/{clean_doi}");

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    let response = match agent
        .get(&url)
        .set("User-Agent", "scientist-in-loop/0.1.0")
        .call()
    {
        Ok(res) => res,
        Err(ureq::Error::Status(404, _)) => return Ok(None),
        Err(e) => {
            return Err(ParseError::Message(format!(
                "Crossref DOI fetch error: {e}"
            )));
        }
    };

    let json: serde_json::Value = response
        .into_json()
        .map_err(|e| ParseError::Message(format!("Failed to parse Crossref DOI JSON: {e}")))?;

    if let Some(message) = json.get("message") {
        Ok(parse_crossref_item(message))
    } else {
        Ok(None)
    }
}

/// Fetch top journal publications matching a query using native Rust Crossref API as primary source.
pub fn fetch_journal_publications(
    query: &str,
    limit: usize,
    script_path: Option<&Utf8Path>,
    python_bin: Option<&str>,
) -> Result<Vec<JournalPublication>, ParseError> {
    if script_path.is_some() {
        return fetch_journal_publications_python(query, limit, script_path, python_bin);
    }

    match fetch_journal_publications_native(query, limit) {
        Ok(pubs) if !pubs.is_empty() => Ok(pubs),
        _ => fetch_journal_publications_python(query, limit, script_path, python_bin),
    }
}

/// Fallback runner calling Python script `fetch_journal_digest.py`.
pub fn fetch_journal_publications_python(
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
        return Err(ParseError::Marker(format!(
            "fetch_journal_digest.py failed: {stderr}"
        )));
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
        std::fs::write(
            &script_path,
            "import sys; sys.stderr.write('API Error'); sys.exit(1)",
        )
        .unwrap();

        let path = Utf8PathBuf::from_path_buf(script_path).unwrap();
        let err =
            fetch_journal_publications("quantum", 5, Some(&path), Some("python3")).unwrap_err();
        assert!(err.to_string().contains("API Error"));
    }

    #[test]
    fn test_parse_crossref_item_json() {
        let json = serde_json::json!({
            "DOI": "10.1038/s41586-020-1234-y",
            "title": ["Quantum Supremacy in a Programmable Processor"],
            "author": [
                {"given": "Frank", "family": "Arute"},
                {"given": "Kunald", "family": "Arya"}
            ],
            "container-title": ["Nature"],
            "published": {
                "date-parts": [[2019, 10, 23]]
            },
            "abstract": "<jats:p>The promise of quantum computers...</jats:p>",
            "is-referenced-by-count": 1500,
            "URL": "https://doi.org/10.1038/s41586-020-1234-y",
            "link": [
                {"URL": "https://nature.com/articles/s41586-020-1234-y.pdf", "content-type": "application/pdf"}
            ]
        });

        let pub_item = parse_crossref_item(&json).expect("should parse valid crossref item");
        assert_eq!(pub_item.doi.as_deref(), Some("10.1038/s41586-020-1234-y"));
        assert_eq!(
            pub_item.title,
            "Quantum Supremacy in a Programmable Processor"
        );
        assert_eq!(pub_item.authors, "Frank Arute, Kunald Arya");
        assert_eq!(pub_item.journal, "Nature");
        assert_eq!(pub_item.year, Some(2019));
        assert_eq!(
            pub_item.abstract_text,
            "The promise of quantum computers..."
        );
        assert_eq!(pub_item.citation_count, Some(1500));
        assert_eq!(
            pub_item.pdf_url.as_deref(),
            Some("https://nature.com/articles/s41586-020-1234-y.pdf")
        );
    }
}
