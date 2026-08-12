//! OpenReview API lookups and BibTeX retrieval.

use crate::doi::DoiMetadataResult;
use crate::error::ApiError;
use crate::ratelimit::enforce_api_ratelimit;
use crate::retry::with_retry;

/// Clean input string by extracting and returning the OpenReview note ID.
///
/// Strips OpenReview URL prefixes (`https://openreview.net/forum?id=`, `https://openreview.net/pdf?id=`),
/// `openreview:`, surrounding whitespace, and trailing punctuation.
pub fn clean_openreview_id_str(input: &str) -> String {
    let s = input.trim();
    if let Some(id) = sil_regex::extract_openreview_id(s) {
        return id;
    }
    let mut cleaned = s.trim_matches(&[' ', '<', '>', '.', ',', ';', ')', ']'][..]);
    if let Some(pos) = cleaned.find("openreview.net/forum?id=") {
        cleaned = cleaned[pos + 24..].trim();
    } else if let Some(pos) = cleaned.find("openreview.net/pdf?id=") {
        cleaned = cleaned[pos + 22..].trim();
    }
    if let Some(stripped) = cleaned.strip_prefix("openreview:") {
        cleaned = stripped.trim();
    } else if let Some(stripped) = cleaned.strip_prefix("OpenReview:") {
        cleaned = stripped.trim();
    }
    if let Some(pos) = cleaned.find('&') {
        cleaned = &cleaned[..pos];
    }
    cleaned
        .trim_matches(&[' ', '<', '>', '.', ',', ';', ')', ']'][..])
        .to_string()
}

/// Helper function to extract title from OpenReview note JSON structure.
///
/// Supports both OpenReview v2 format (`notes[0]["content"]["title"]["value"]`)
/// and v1 format (`notes[0]["content"]["title"]`).
pub(crate) fn extract_title_from_openreview_note(note: &serde_json::Value) -> Option<String> {
    let content = note.get("content")?;
    let title_val = content.get("title")?;
    if let Some(val) = title_val.get("value").and_then(|v| v.as_str()) {
        let t = val.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    if let Some(val) = title_val.as_str() {
        let t = val.trim();
        if !t.is_empty() {
            return Some(t.to_string());
        }
    }
    None
}

/// Verify OpenReview note existence and retrieve paper title metadata.
///
/// Enforces rate limiting, queries `https://api2.openreview.net/notes?id={clean_id}`
/// (falling back to v1 `https://api.openreview.net/notes?id={clean_id}`).
///
/// Returns `Ok(DoiMetadataResult { exists: true, title: Some(extracted_title) })` if note exists,
/// `Ok(DoiMetadataResult { exists: false, title: None })` for 404 or empty notes list,
/// or `Err(ApiError)` for transport/network/rate-limit failures.
pub fn verify_openreview_with_metadata(or_id: &str) -> Result<DoiMetadataResult, ApiError> {
    let clean_id = clean_openreview_id_str(or_id);
    if clean_id.is_empty() {
        return Ok(DoiMetadataResult {
            exists: false,
            title: None,
        });
    }

    enforce_api_ratelimit();

    let agent = ureq::AgentBuilder::new()
        .timeout(std::time::Duration::from_secs(10))
        .build();

    // Query API v2 first
    let v2_url = format!("https://api2.openreview.net/notes?id={clean_id}");
    let v2_res = agent
        .get(&v2_url)
        .set(
            "User-Agent",
            "scientist-in-loop/0.1.0 (mailto:info@scientist-in-loop.org)",
        )
        .call();

    match v2_res {
        Ok(res) => {
            if (200..=299).contains(&res.status())
                && let Ok(json) = res.into_json::<serde_json::Value>()
                && let Some(notes) = json.get("notes").and_then(|n| n.as_array())
                && !notes.is_empty()
            {
                let title = extract_title_from_openreview_note(&notes[0]);
                return Ok(DoiMetadataResult {
                    exists: true,
                    title,
                });
            }
        }
        Err(ureq::Error::Status(429, _)) => {
            return Err(ApiError::RateLimited(format!(
                "Rate limited (429) verifying OpenReview ID '{clean_id}'"
            )));
        }
        Err(ureq::Error::Status(_, _)) => {}
        Err(ureq::Error::Transport(_)) => {}
    }

    // Fallback to API v1
    enforce_api_ratelimit();
    let v1_url = format!("https://api.openreview.net/notes?id={clean_id}");
    let v1_res = agent
        .get(&v1_url)
        .set(
            "User-Agent",
            "scientist-in-loop/0.1.0 (mailto:info@scientist-in-loop.org)",
        )
        .call();

    match v1_res {
        Ok(res) => {
            if (200..=299).contains(&res.status())
                && let Ok(json) = res.into_json::<serde_json::Value>()
                && let Some(notes) = json.get("notes").and_then(|n| n.as_array())
                && !notes.is_empty()
            {
                let title = extract_title_from_openreview_note(&notes[0]);
                return Ok(DoiMetadataResult {
                    exists: true,
                    title,
                });
            }
            Ok(DoiMetadataResult {
                exists: false,
                title: None,
            })
        }
        Err(ureq::Error::Status(404, _)) => Ok(DoiMetadataResult {
            exists: false,
            title: None,
        }),
        Err(ureq::Error::Status(429, _)) => Err(ApiError::RateLimited(format!(
            "Rate limited (429) verifying OpenReview ID '{clean_id}'"
        ))),
        Err(ureq::Error::Status(status, _)) => Err(ApiError::NetworkError(format!(
            "HTTP status {status} verifying OpenReview ID '{clean_id}'"
        ))),
        Err(ureq::Error::Transport(e)) => Err(ApiError::NetworkError(format!(
            "Network error verifying OpenReview ID '{clean_id}': {e}"
        ))),
    }
}

/// Fetch official BibTeX string from OpenReview by note ID.
pub fn fetch_bibtex_by_openreview_id(or_id: &str) -> Result<Option<String>, ApiError> {
    let clean_id = clean_openreview_id_str(or_id);
    if clean_id.is_empty() {
        return Ok(None);
    }

    with_retry(|| {
        enforce_api_ratelimit();

        let url = format!("https://openreview.net/bibtex?id={clean_id}");
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(8))
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
                    "Rate limited (429) fetching OpenReview BibTeX for '{clean_id}'"
                )));
            }
            Err(ureq::Error::Status(status, _)) => {
                return Err(ApiError::NetworkError(format!(
                    "HTTP status {status} fetching OpenReview BibTeX for '{clean_id}'"
                )));
            }
            Err(ureq::Error::Transport(e)) => {
                return Err(ApiError::NetworkError(format!(
                    "Network error fetching OpenReview BibTeX for '{clean_id}': {e}"
                )));
            }
        };

        let body = response
            .into_string()
            .map_err(|e| ApiError::ParseError(format!("Failed to read OpenReview BibTeX: {e}")))?;

        let trimmed = body.trim();
        if trimmed
            .lines()
            .any(|l| l.trim_start().starts_with('@') && l.contains('{'))
        {
            Ok(Some(sil_core::bib::pretty_format_bibtex(trimmed)))
        } else {
            Ok(None)
        }
    })
}
