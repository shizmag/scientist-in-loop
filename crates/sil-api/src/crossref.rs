use crate::doi::clean_doi_str;
use crate::error::ApiError;
use crate::ratelimit::enforce_api_ratelimit;
use crate::retry::with_retry;
use sil_core::JournalPublication;
use std::collections::HashSet;

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

fn urlencode(s: &str) -> String {
    let mut encoded = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            b' ' => encoded.push_str("%20"),
            _ => encoded.push_str(&format!("%{:02X}", byte)),
        }
    }
    encoded
}

/// Build Crossref API request URL for fetching top journal publications digest (`filter=type:journal-article`).
pub fn build_crossref_digest_url(query: &str, limit: usize) -> String {
    format!(
        "https://api.crossref.org/works?query={}&filter=type:journal-article&rows={}&sort=relevance",
        urlencode(query),
        limit
    )
}

/// Fetch publications directly from Crossref API natively in Rust using `ureq`.
pub fn fetch_journal_publications_native(
    query: &str,
    limit: usize,
) -> Result<Vec<JournalPublication>, ApiError> {
    with_retry(|| {
        enforce_api_ratelimit();
        let url = build_crossref_digest_url(query, limit);

        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();

        let response = agent
            .get(&url)
            .set(
                "User-Agent",
                "scientist-in-loop/0.1.0 (mailto:info@scientist-in-loop.org)",
            )
            .call()
            .map_err(|e| match e {
                ureq::Error::Status(429, _) => {
                    ApiError::RateLimited("Crossref API rate limit exceeded".into())
                }
                ureq::Error::Status(404, _) => {
                    ApiError::NotFound("Crossref publications endpoint not found".into())
                }
                ureq::Error::Status(s, _) => ApiError::NetworkError(format!("HTTP status {s}")),
                ureq::Error::Transport(t) => {
                    ApiError::NetworkError(format!("Network transport error: {t}"))
                }
            })?;

        let json: serde_json::Value = response.into_json().map_err(|e| {
            ApiError::ParseError(format!("Failed to parse Crossref response JSON: {e}"))
        })?;

        let items = json
            .get("message")
            .and_then(|m| m.get("items"))
            .and_then(|i| i.as_array())
            .ok_or_else(|| {
                ApiError::ParseError("Invalid Crossref API payload structure".to_string())
            })?;

        let mut publications = Vec::new();
        for item in items {
            if let Some(pub_item) = parse_crossref_item(item) {
                publications.push(pub_item);
            }
        }

        Ok(publications)
    })
}

/// Fetch single paper's metadata from Crossref API natively in Rust using `ureq`.
pub fn fetch_work_by_doi(doi: &str) -> Result<Option<JournalPublication>, ApiError> {
    let clean_doi = clean_doi_str(doi);
    if clean_doi.is_empty() {
        return Ok(None);
    }

    with_retry(|| {
        enforce_api_ratelimit();
        let url = format!("https://api.crossref.org/works/{clean_doi}");

        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();

        let response = match agent
            .get(&url)
            .set(
                "User-Agent",
                "scientist-in-loop/0.1.0 (mailto:info@scientist-in-loop.org)",
            )
            .call()
        {
            Ok(res) => res,
            Err(ureq::Error::Status(404, _)) => return Ok(None),
            Err(ureq::Error::Status(429, _)) => {
                return Err(ApiError::RateLimited(format!(
                    "Rate limited (429) fetching work for DOI '{clean_doi}'"
                )));
            }
            Err(ureq::Error::Status(s, _)) => {
                return Err(ApiError::NetworkError(format!(
                    "HTTP status {s} fetching work for DOI '{clean_doi}'"
                )));
            }
            Err(ureq::Error::Transport(t)) => {
                return Err(ApiError::NetworkError(format!(
                    "Network transport error fetching work for DOI '{clean_doi}': {t}"
                )));
            }
        };

        let json: serde_json::Value = response
            .into_json()
            .map_err(|e| ApiError::ParseError(format!("Failed to parse Crossref DOI JSON: {e}")))?;

        if let Some(message) = json.get("message") {
            Ok(parse_crossref_item(message))
        } else {
            Ok(None)
        }
    })
}

fn tokenize_title(title: &str) -> HashSet<String> {
    title
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .map(|s| s.to_string())
        .collect()
}

/// Compute token-based Jaccard similarity between two title strings.
/// Returns a score between 0.0 and 1.0.
pub fn title_similarity(a: &str, b: &str) -> f64 {
    let tokens_a = tokenize_title(a);
    let tokens_b = tokenize_title(b);

    if tokens_a.is_empty() && tokens_b.is_empty() {
        return 1.0;
    }
    if tokens_a.is_empty() || tokens_b.is_empty() {
        return 0.0;
    }

    let intersection = tokens_a.intersection(&tokens_b).count();
    let union = tokens_a.union(&tokens_b).count();

    if union == 0 {
        0.0
    } else {
        intersection as f64 / union as f64
    }
}

/// Detailed outcome of DOI lookup by title from Crossref.
#[derive(Debug, Clone, PartialEq)]
pub enum TitleLookupOutcome {
    /// Match found with similarity score above or equal to threshold (0.6).
    Match {
        /// Resolved DOI.
        doi: String,
        /// Title returned by Crossref.
        title: String,
        /// Similarity score.
        similarity: f64,
    },
    /// Match found, but rejected due to low confidence similarity (< 0.6).
    LowConfidence {
        /// Title returned by Crossref.
        found_title: String,
        /// Similarity score.
        similarity: f64,
    },
    /// No item returned by Crossref.
    NoMatch,
}

/// Lookup DOI for paper title with detailed outcome including title similarity checking against Crossref results.
pub fn lookup_doi_by_title_detailed(
    title: &str,
    authors: Option<&str>,
) -> Result<TitleLookupOutcome, ApiError> {
    enforce_api_ratelimit();
    let clean_title = title.trim();
    if clean_title.is_empty() {
        return Ok(TitleLookupOutcome::NoMatch);
    }

    let query = if let Some(a) = authors {
        format!("{clean_title} {a}")
    } else {
        clean_title.to_string()
    };

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(8))
        .build();

    let response = match agent
        .get("https://api.crossref.org/works")
        .set(
            "User-Agent",
            "scientist-in-loop/0.1.0 (mailto:info@scientist-in-loop.org)",
        )
        .query("query.bibliographic", &query)
        .query("rows", "1")
        .call()
    {
        Ok(res) => res,
        Err(ureq::Error::Status(404, _)) => return Ok(TitleLookupOutcome::NoMatch),
        Err(ureq::Error::Status(429, _)) => {
            return Err(ApiError::RateLimited(
                "Rate limit exceeded on Crossref title lookup".into(),
            ));
        }
        Err(e) => {
            return Err(ApiError::NetworkError(format!(
                "Crossref title lookup request failed: {e}"
            )));
        }
    };

    let json: serde_json::Value = response
        .into_json()
        .map_err(|e| ApiError::ParseError(format!("Failed to parse title lookup JSON: {e}")))?;

    if let Some(first_item) = json
        .get("message")
        .and_then(|m| m.get("items"))
        .and_then(|i| i.as_array())
        .and_then(|a| a.first())
        && let Some(doi) = first_item.get("DOI").and_then(|d| d.as_str())
    {
        let found_title = first_item
            .get("title")
            .and_then(|v| v.as_array())
            .and_then(|arr| arr.first())
            .and_then(|v| v.as_str())
            .unwrap_or("");

        let sim = title_similarity(clean_title, found_title);
        if sim >= 0.6 {
            return Ok(TitleLookupOutcome::Match {
                doi: doi.to_string(),
                title: found_title.to_string(),
                similarity: sim,
            });
        } else {
            return Ok(TitleLookupOutcome::LowConfidence {
                found_title: found_title.to_string(),
                similarity: sim,
            });
        }
    }

    Ok(TitleLookupOutcome::NoMatch)
}

/// Lookup DOI for a paper title and optional author list using Crossref API.
/// Rejects matches with title similarity below 0.6 threshold.
pub fn lookup_doi_by_title(title: &str, authors: Option<&str>) -> Result<Option<String>, ApiError> {
    match lookup_doi_by_title_detailed(title, authors)? {
        TitleLookupOutcome::Match { doi, .. } => Ok(Some(doi)),
        TitleLookupOutcome::LowConfidence { .. } | TitleLookupOutcome::NoMatch => Ok(None),
    }
}
