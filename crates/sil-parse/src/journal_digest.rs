//! Top-journal publication digest feed and Crossref metadata hydration natively in Rust.

use crate::error::ParseError;
use camino::{Utf8Path, Utf8PathBuf};
use sil_core::JournalPublication;
use std::process::Command;

fn map_api_err(e: sil_api::ApiError) -> ParseError {
    ParseError::Message(e.to_string())
}

pub use sil_api::{
    build_crossref_digest_url, parse_crossref_item, title_similarity, TitleLookupOutcome,
};

/// Enforce a minimal rate-limiting delay (250ms) between external HTTP API requests.
pub fn enforce_api_ratelimit() {
    sil_api::enforce_api_ratelimit();
}

/// Fetch publications directly from Crossref API natively in Rust using `ureq`.
pub fn fetch_journal_publications_native(
    query: &str,
    limit: usize,
) -> Result<Vec<JournalPublication>, ParseError> {
    sil_api::fetch_journal_publications_native(query, limit).map_err(map_api_err)
}

/// Fetch single paper's metadata from Crossref API natively in Rust using `ureq`.
pub fn fetch_work_by_doi(doi: &str) -> Result<Option<JournalPublication>, ParseError> {
    sil_api::fetch_work_by_doi(doi).map_err(map_api_err)
}

/// Fetch official BibTeX string from DOI content negotiation (`https://doi.org/{doi}`).
pub fn fetch_bibtex_by_doi(doi: &str) -> Result<Option<String>, ParseError> {
    sil_api::fetch_bibtex_by_doi(doi).map_err(map_api_err)
}

/// Lookup DOI for paper title with detailed outcome including title similarity checking against Crossref results.
pub fn lookup_doi_by_title_detailed(
    title: &str,
    authors: Option<&str>,
) -> Result<TitleLookupOutcome, ParseError> {
    sil_api::lookup_doi_by_title_detailed(title, authors).map_err(map_api_err)
}

/// Lookup DOI for a paper title and optional author list using Crossref API.
/// Rejects matches with title similarity below 0.6 threshold.
pub fn lookup_doi_by_title(
    title: &str,
    authors: Option<&str>,
) -> Result<Option<String>, ParseError> {
    sil_api::lookup_doi_by_title(title, authors).map_err(map_api_err)
}

/// Fetch official BibTeX string directly from arXiv API (`https://arxiv.org/bibtex/{clean_id}`).
pub fn fetch_bibtex_by_arxiv_id(arxiv_id: &str) -> Result<Option<String>, ParseError> {
    sil_api::fetch_bibtex_by_arxiv_id(arxiv_id).map_err(map_api_err)
}

/// Fetch paper metadata by arXiv ID (e.g. `2405.12345` or `arXiv:2405.12345v1`) from arXiv API.
pub fn fetch_work_by_arxiv_id(arxiv_id: &str) -> Result<Option<JournalPublication>, ParseError> {
    sil_api::fetch_work_by_arxiv_id(arxiv_id).map_err(map_api_err)
}

/// Result of resolving official BibTeX metadata for a ReferenceEntry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReferenceBibResolution {
    /// Official BibTeX string fetched via DOI content negotiation, arXiv API, or Crossref lookup.
    Resolved(String),
    /// Metadata resolution failed with clear explanation.
    Failed(String),
}

/// Resolve official BibTeX metadata for a reference entry via DOI content negotiation, arXiv API, & Crossref lookup.
/// Returns `ReferenceBibResolution::Failed` with a reason if official metadata could not be fetched.
pub fn resolve_official_bibtex_entry(entry: &sil_core::ReferenceEntry) -> ReferenceBibResolution {
    let mut reasons = Vec::new();

    // 1. Try DOI fetch if present
    if let Some(ref doi) = entry.doi {
        let clean_doi = doi
            .trim_start_matches("doi:")
            .trim_start_matches("https://doi.org/")
            .trim_start_matches("http://doi.org/")
            .trim();
        if !clean_doi.is_empty() {
            match fetch_bibtex_by_doi(clean_doi) {
                Ok(Some(bib)) => return ReferenceBibResolution::Resolved(bib),
                Ok(None) => reasons.push(format!("DOI '{clean_doi}' returned no BibTeX")),
                Err(e) => reasons.push(format!("DOI '{clean_doi}' fetch failed: {e}")),
            }
        }
    }

    // 2. Try arXiv ID fetch if present or extractable from DOI/title
    let arxiv_candidate = entry
        .arxiv_id
        .as_deref()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .or_else(|| entry.doi.as_deref().and_then(sil_regex::extract_arxiv_id))
        .or_else(|| entry.title.as_deref().and_then(sil_regex::extract_arxiv_id));

    if let Some(arxiv_id) = arxiv_candidate {
        let clean_id = arxiv_id
            .trim_start_matches("arxiv:")
            .trim_start_matches("arXiv:")
            .trim();
        if !clean_id.is_empty() {
            match fetch_bibtex_by_arxiv_id(clean_id) {
                Ok(Some(bib)) => return ReferenceBibResolution::Resolved(bib),
                Ok(None) => reasons.push(format!("arXiv ID '{clean_id}' returned no BibTeX")),
                Err(e) => reasons.push(format!("arXiv ID '{clean_id}' fetch failed: {e}")),
            }
        }
    }

    // 3. Try title (+ authors) lookup via Crossref to find DOI
    if let Some(ref title) = entry.title {
        let clean_title = title.trim();
        if !clean_title.is_empty() {
            match lookup_doi_by_title_detailed(clean_title, entry.authors.as_deref()) {
                Ok(TitleLookupOutcome::Match { doi, similarity, .. }) => {
                    match fetch_bibtex_by_doi(&doi) {
                        Ok(Some(bib)) => return ReferenceBibResolution::Resolved(bib),
                        Ok(None) => reasons.push(format!(
                            "Crossref found DOI '{doi}' (similarity {similarity:.2}) for title '{clean_title}', but BibTeX fetch returned no entry"
                        )),
                        Err(e) => reasons.push(format!(
                            "Crossref found DOI '{doi}' (similarity {similarity:.2}) for title '{clean_title}', but BibTeX fetch failed: {e}"
                        )),
                    }
                }
                Ok(TitleLookupOutcome::LowConfidence { found_title, similarity }) => {
                    reasons.push(format!(
                        "Crossref match '{found_title}' for title '{clean_title}' rejected due to low confidence (similarity {similarity:.2} < 0.60)"
                    ));
                }
                Ok(TitleLookupOutcome::NoMatch) => {
                    reasons.push(format!("No Crossref match found for title '{clean_title}'"));
                }
                Err(e) => {
                    reasons.push(format!("Crossref lookup failed for title '{clean_title}': {e}"));
                }
            }
        }
    }

    // 4. Missing required metadata or all attempts failed
    if reasons.is_empty() {
        let mut missing = Vec::new();
        if entry.doi.is_none() {
            missing.push("DOI");
        }
        if entry.arxiv_id.is_none() {
            missing.push("arXiv ID");
        }
        if entry.title.is_none() {
            missing.push("title");
        }
        ReferenceBibResolution::Failed(format!(
            "Missing required metadata to fetch official BibTeX ({})",
            missing.join(", ")
        ))
    } else {
        ReferenceBibResolution::Failed(reasons.join("; "))
    }
}

/// Resolve official, 100% accurate BibTeX metadata for a reference entry via DOI content negotiation, arXiv API, & Crossref lookup,
/// falling back to local `entry.to_bibtex()` if network is unavailable or no match is found.
pub fn resolve_official_bibtex(entry: &sil_core::ReferenceEntry) -> String {
    match resolve_official_bibtex_entry(entry) {
        ReferenceBibResolution::Resolved(bib) => bib,
        ReferenceBibResolution::Failed(_) => entry.to_bibtex(),
    }
}

/// Result of resolving official BibTeX metadata for a source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SourceBibResolution {
    /// Official BibTeX string fetched via DOI content negotiation, arXiv API, or Crossref lookup.
    Resolved(String),
    /// Metadata resolution failed with clear explanation.
    Failed(String),
}

/// Resolve official BibTeX metadata for a `SourceDocument` via DOI content negotiation, arXiv API, or Crossref lookup.
/// Returns `SourceBibResolution::Failed` with a reason if official metadata could not be fetched.
pub fn resolve_official_bibtex_for_source(doc: &sil_core::SourceDocument) -> SourceBibResolution {
    let mut reasons = Vec::new();

    // 1. Try DOI fetch if present
    if let Some(ref doi) = doc.doi {
        let clean_doi = doi
            .trim_start_matches("doi:")
            .trim_start_matches("https://doi.org/")
            .trim_start_matches("http://doi.org/")
            .trim();
        if !clean_doi.is_empty() {
            match fetch_bibtex_by_doi(clean_doi) {
                Ok(Some(bib)) => return SourceBibResolution::Resolved(bib),
                Ok(None) => reasons.push(format!("DOI '{clean_doi}' returned no BibTeX")),
                Err(e) => reasons.push(format!("DOI '{clean_doi}' fetch failed: {e}")),
            }
        }
    }

    // 2. Check for arXiv ID in DOI, filename, or title
    let arxiv_candidate = doc
        .doi
        .as_deref()
        .and_then(sil_regex::extract_arxiv_id)
        .or_else(|| sil_regex::extract_arxiv_id(&doc.filename))
        .or_else(|| doc.title.as_deref().and_then(sil_regex::extract_arxiv_id));

    if let Some(arxiv_id) = arxiv_candidate {
        let clean_id = arxiv_id
            .trim_start_matches("arxiv:")
            .trim_start_matches("arXiv:")
            .trim();
        if !clean_id.is_empty() {
            match fetch_bibtex_by_arxiv_id(clean_id) {
                Ok(Some(bib)) => return SourceBibResolution::Resolved(bib),
                Ok(None) => reasons.push(format!("arXiv ID '{clean_id}' returned no BibTeX")),
                Err(e) => reasons.push(format!("arXiv ID '{clean_id}' fetch failed: {e}")),
            }
        }
    }

    // 3. Try Crossref lookup by title (+ authors) to find DOI
    if let Some(ref title) = doc.title {
        let clean_title = title.trim();
        if !clean_title.is_empty() {
            match lookup_doi_by_title_detailed(clean_title, doc.authors.as_deref()) {
                Ok(TitleLookupOutcome::Match { doi, similarity, .. }) => {
                    match fetch_bibtex_by_doi(&doi) {
                        Ok(Some(bib)) => return SourceBibResolution::Resolved(bib),
                        Ok(None) => reasons.push(format!(
                            "Crossref found DOI '{doi}' (similarity {similarity:.2}) for title '{clean_title}', but BibTeX fetch returned no entry"
                        )),
                        Err(e) => reasons.push(format!(
                            "Crossref found DOI '{doi}' (similarity {similarity:.2}) for title '{clean_title}', but BibTeX fetch failed: {e}"
                        )),
                    }
                }
                Ok(TitleLookupOutcome::LowConfidence { found_title, similarity }) => {
                    reasons.push(format!(
                        "Crossref match '{found_title}' for title '{clean_title}' rejected due to low confidence (similarity {similarity:.2} < 0.60)"
                    ));
                }
                Ok(TitleLookupOutcome::NoMatch) => {
                    reasons.push(format!("No Crossref match found for title '{clean_title}'"));
                }
                Err(e) => {
                    reasons.push(format!("Crossref lookup failed for title '{clean_title}': {e}"));
                }
            }
        }
    }

    // 4. Missing necessary metadata or all attempts failed
    if reasons.is_empty() {
        let mut missing = Vec::new();
        if doc.doi.is_none() {
            missing.push("DOI");
        }
        if doc.title.is_none() {
            missing.push("title");
        }
        SourceBibResolution::Failed(format!(
            "Missing required metadata to fetch official BibTeX ({})",
            missing.join(", ")
        ))
    } else {
        SourceBibResolution::Failed(reasons.join("; "))
    }
}

/// Fetch top journal publications matching a query using native Rust Crossref API as primary source.
pub fn fetch_journal_publications(
    query: &str,
    limit: usize,
    script_path: Option<&Utf8Path>,
    python_bin: Option<&str>,
) -> Result<Vec<JournalPublication>, ParseError> {
    if script_path.is_none() {
        match fetch_journal_publications_native(query, limit) {
            Ok(pubs) if !pubs.is_empty() => return Ok(pubs),
            _ => {}
        }
    }

    fetch_journal_publications_python(query, limit, script_path, python_bin)
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

    #[test]
    fn test_parse_crossref_item_doi_fallback_url() {
        let json = serde_json::json!({
            "DOI": "10.1016/j.cell.2023.01.001",
            "title": ["Cell Biology Paper"]
        });

        let pub_item = parse_crossref_item(&json).expect("should parse");
        assert_eq!(pub_item.url, "https://doi.org/10.1016/j.cell.2023.01.001");
    }

    #[test]
    fn test_parse_crossref_item_missing_title_and_doi() {
        let json = serde_json::json!({
            "author": [{"given": "Bob"}]
        });
        assert!(parse_crossref_item(&json).is_none());
    }

    #[test]
    fn test_enforce_api_ratelimit() {
        enforce_api_ratelimit();
        enforce_api_ratelimit();
    }

    #[test]
    fn test_fetch_work_by_arxiv_id_empty() {
        let res = fetch_work_by_arxiv_id("  ").unwrap();
        assert!(res.is_none());
    }

    #[test]
    fn test_resolve_official_bibtex_fallback() {
        let entry = sil_core::ReferenceEntry {
            id: "ref-1".to_string(),
            source_id: sil_core::SourceId::new("paper.pdf"),
            ref_index: 1,
            raw_text: "Vaswani et al. Attention is all you need. 2017.".to_string(),
            authors: Some("Vaswani et al.".to_string()),
            title: Some("Attention is all you need".to_string()),
            year: Some(2017),
            venue: None,
            doi: None,
            arxiv_id: None,
            url: None,
        };
        let bib = resolve_official_bibtex(&entry);
        assert!(bib.contains("@article{"));
        assert!(bib.to_lowercase().contains("attention is all you need"));
    }

    #[test]
    fn test_resolve_official_bibtex_for_source_missing_metadata() {
        let mut doc = sil_core::SourceDocument::new("unparsed_file.pdf".into());
        doc.doi = None;
        doc.title = None;
        let res = resolve_official_bibtex_for_source(&doc);
        match res {
            SourceBibResolution::Failed(reason) => {
                assert!(reason.contains("Missing required metadata"));
            }
            SourceBibResolution::Resolved(_) => panic!("Expected failed resolution"),
        }
    }

    #[test]
    fn test_title_similarity_function() {
        // Identical titles
        assert_eq!(
            title_similarity("Attention Is All You Need", "Attention Is All You Need"),
            1.0
        );

        // Case & punctuation insensitivity
        assert_eq!(
            title_similarity("Attention Is All You Need!", "attention is all you need."),
            1.0
        );

        // Empty titles
        assert_eq!(title_similarity("", ""), 1.0);
        assert_eq!(title_similarity("Some Title", ""), 0.0);

        // High similarity (minor differences / extra words)
        let sim = title_similarity(
            "Attention Is All You Need",
            "Attention Is All You Need for Deep Learning",
        );
        assert!(sim >= 0.60, "Expected sim >= 0.60, got {sim}");

        // Low similarity
        let low_sim = title_similarity("Attention Is All You Need", "Quantum Supremacy Processor");
        assert!(low_sim < 0.60, "Expected low_sim < 0.60, got {low_sim}");
    }

    #[test]
    fn test_resolve_official_bibtex_entry_fallback_chain_on_error() {
        let entry = sil_core::ReferenceEntry {
            id: "ref-test".to_string(),
            source_id: sil_core::SourceId::new("paper.pdf"),
            ref_index: 1,
            raw_text: "Test entry with bad DOI".to_string(),
            authors: Some("A. Scientist".to_string()),
            title: Some("Some Random Nonexistent Paper Title XYZ".to_string()),
            year: Some(2023),
            venue: None,
            doi: Some("10.0000/invalid-doi-test-nonexistent".to_string()),
            arxiv_id: Some("0000.00000".to_string()),
            url: None,
        };

        let res = resolve_official_bibtex_entry(&entry);
        match res {
            ReferenceBibResolution::Failed(reason) => {
                assert!(
                    reason.contains("DOI '10.0000/invalid-doi-test-nonexistent'"),
                    "Reason should mention DOI attempt: {reason}"
                );
                assert!(
                    reason.contains("arXiv ID '0000.00000'"),
                    "Reason should mention arXiv ID attempt: {reason}"
                );
            }
            ReferenceBibResolution::Resolved(_) => panic!("Expected failed resolution"),
        }
    }

    #[test]
    fn test_resolve_official_bibtex_for_source_fallback_chain_on_error() {
        let mut doc = sil_core::SourceDocument::new("0000.00000.pdf".into());
        doc.doi = Some("10.0000/invalid-doi-test-nonexistent".to_string());
        doc.title = Some("Some Random Nonexistent Paper Title XYZ".to_string());

        let res = resolve_official_bibtex_for_source(&doc);
        match res {
            SourceBibResolution::Failed(reason) => {
                assert!(
                    reason.contains("DOI '10.0000/invalid-doi-test-nonexistent'"),
                    "Reason should mention DOI attempt: {reason}"
                );
                assert!(
                    reason.contains("arXiv ID '0000.00000'"),
                    "Reason should mention arXiv ID attempt: {reason}"
                );
            }
            SourceBibResolution::Resolved(_) => panic!("Expected failed resolution"),
        }
    }
    #[test]
    fn test_build_crossref_digest_url_parameters() {
        let url = build_crossref_digest_url("quantum computing", 10);
        assert!(url.starts_with("https://api.crossref.org/works"));
        assert!(url.contains("query=quantum%20computing"));
        assert!(url.contains("filter=type:journal-article"));
        assert!(url.contains("rows=10"));
        assert!(url.contains("sort=relevance"));
    }
}
