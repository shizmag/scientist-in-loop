use crate::error::ApiError;
use crate::ratelimit::enforce_api_ratelimit;
use crate::retry::with_retry;

/// Result of verifying a DOI with metadata from Crossref.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DoiMetadataResult {
    /// Whether the DOI exists in Crossref.
    pub exists: bool,
    /// Official title of the paper if available in metadata.
    pub title: Option<String>,
}

/// Clean DOI string by stripping prefixes (`doi:`, `DOI:`, `https://doi.org/`, `http://doi.org/`, `http://dx.doi.org/`, `https://dx.doi.org/`) and surrounding whitespace.
pub fn clean_doi_str(doi: &str) -> String {
    let mut s = doi.trim();
    let mut changed = true;
    while changed {
        changed = false;
        let prev = s;
        if let Some(stripped) = s.strip_prefix("https://doi.org/") {
            s = stripped.trim();
            changed = true;
        } else if let Some(stripped) = s.strip_prefix("http://doi.org/") {
            s = stripped.trim();
            changed = true;
        } else if let Some(stripped) = s.strip_prefix("http://dx.doi.org/") {
            s = stripped.trim();
            changed = true;
        } else if let Some(stripped) = s.strip_prefix("https://dx.doi.org/") {
            s = stripped.trim();
            changed = true;
        } else if let Some(stripped) = s.strip_prefix("doi:") {
            s = stripped.trim();
            changed = true;
        } else if let Some(stripped) = s.strip_prefix("DOI:") {
            s = stripped.trim();
            changed = true;
        }
        if s != prev {
            changed = true;
        }
    }
    s.to_string()
}

/// Check if a DOI exists via Crossref API.
///
/// Returns `Ok(true)` for 200/2xx, `Ok(false)` for 404 or empty DOI,
/// or `Err(ApiError::NetworkError(...))` (or `ApiError::RateLimited`) for errors.
pub fn check_doi_exists(doi: &str) -> Result<bool, ApiError> {
    let clean_doi = clean_doi_str(doi);
    if clean_doi.is_empty() {
        return Ok(false);
    }

    with_retry(|| {
        enforce_api_ratelimit();

        let url = format!("https://api.crossref.org/works/{clean_doi}");
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(10))
            .build();

        match agent
            .get(&url)
            .set(
                "User-Agent",
                "scientist-in-loop/0.1.0 (mailto:info@scientist-in-loop.org)",
            )
            .call()
        {
            Ok(res) => {
                let status = res.status();
                if (200..=299).contains(&status) {
                    Ok(true)
                } else {
                    Err(ApiError::NetworkError(format!(
                        "Unexpected HTTP status {status} checking DOI '{clean_doi}'"
                    )))
                }
            }
            Err(ureq::Error::Status(404, _)) => Ok(false),
            Err(ureq::Error::Status(429, _)) => Err(ApiError::RateLimited(format!(
                "Rate limited (429) checking DOI '{clean_doi}'"
            ))),
            Err(ureq::Error::Status(status, _)) => Err(ApiError::NetworkError(format!(
                "HTTP status {status} checking DOI '{clean_doi}'"
            ))),
            Err(ureq::Error::Transport(e)) => Err(ApiError::NetworkError(format!(
                "Network error checking DOI '{clean_doi}': {e}"
            ))),
        }
    })
}

/// Fetch official BibTeX string from DOI content negotiation (`https://doi.org/{doi}`).
pub fn fetch_bibtex_by_doi(doi: &str) -> Result<Option<String>, ApiError> {
    let clean_doi = clean_doi_str(doi);
    if clean_doi.is_empty() {
        return Ok(None);
    }

    with_retry(|| {
        enforce_api_ratelimit();

        let url = format!("https://doi.org/{clean_doi}");
        let agent = ureq::AgentBuilder::new()
            .timeout(std::time::Duration::from_secs(8))
            .redirects(5)
            .build();

        let response = match agent
            .get(&url)
            .set("Accept", "application/x-bibtex")
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
                    "Rate limited (429) fetching BibTeX for DOI '{clean_doi}'"
                )));
            }
            Err(ureq::Error::Status(status, _)) => {
                return Err(ApiError::NetworkError(format!(
                    "HTTP status {status} fetching BibTeX for DOI '{clean_doi}'"
                )));
            }
            Err(ureq::Error::Transport(e)) => {
                return Err(ApiError::NetworkError(format!(
                    "Network error fetching BibTeX for DOI '{clean_doi}': {e}"
                )));
            }
        };

        let body = response.into_string().map_err(|e| {
            ApiError::ParseError(format!("Failed reading DOI BibTeX response: {e}"))
        })?;

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

/// Extract paper title from Crossref work JSON payload `json["message"]["title"][0]`.
pub(crate) fn extract_title_from_crossref_json(json: &serde_json::Value) -> Option<String> {
    json.get("message")
        .and_then(|m| m.get("title"))
        .and_then(|t| t.as_array())
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

/// Verify DOI existence and retrieve official title metadata via Crossref API (`https://api.crossref.org/works/{clean_doi}`).
///
/// Returns `Ok(DoiMetadataResult { exists: false, title: None })` for empty DOI string or 404 response.
/// Returns `Ok(DoiMetadataResult { exists: true, title: extracted_title })` for 200 OK response.
/// Returns `Err(ApiError::NetworkError(...))` (or `ApiError::RateLimited`) for network/transport failures or HTTP errors.
pub fn verify_doi_with_metadata(doi: &str) -> Result<DoiMetadataResult, ApiError> {
    let clean_doi = clean_doi_str(doi);
    if clean_doi.is_empty() {
        return Ok(DoiMetadataResult {
            exists: false,
            title: None,
        });
    }

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
        Err(ureq::Error::Status(404, _)) => {
            return Ok(DoiMetadataResult {
                exists: false,
                title: None,
            });
        }
        Err(ureq::Error::Status(429, _)) => {
            return Err(ApiError::RateLimited(format!(
                "Rate limited (429) verifying DOI '{clean_doi}'"
            )));
        }
        Err(ureq::Error::Status(status, _)) => {
            return Err(ApiError::NetworkError(format!(
                "HTTP status {status} verifying DOI '{clean_doi}'"
            )));
        }
        Err(ureq::Error::Transport(e)) => {
            return Err(ApiError::NetworkError(format!(
                "Network error verifying DOI '{clean_doi}': {e}"
            )));
        }
    };

    let status = response.status();
    if !(200..=299).contains(&status) {
        return Err(ApiError::NetworkError(format!(
            "Unexpected HTTP status {status} verifying DOI '{clean_doi}'"
        )));
    }

    let json: serde_json::Value = response.into_json().map_err(|e| {
        ApiError::ParseError(format!("Failed to parse Crossref response JSON: {e}"))
    })?;

    let title = extract_title_from_crossref_json(&json);

    Ok(DoiMetadataResult {
        exists: true,
        title,
    })
}
