use crate::error::ApiError;
use crate::ratelimit::enforce_api_ratelimit;

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
}

/// Fetch official BibTeX string from DOI content negotiation (`https://doi.org/{doi}`).
pub fn fetch_bibtex_by_doi(doi: &str) -> Result<Option<String>, ApiError> {
    let clean_doi = clean_doi_str(doi);
    if clean_doi.is_empty() {
        return Ok(None);
    }

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

    let body = response
        .into_string()
        .map_err(|e| ApiError::ParseError(format!("Failed to read DOI BibTeX response string: {e}")))?;

    let trimmed = body.trim();
    if trimmed
        .lines()
        .any(|l| l.trim_start().starts_with('@') && l.contains('{'))
    {
        Ok(Some(sil_core::bib::pretty_format_bibtex(trimmed)))
    } else {
        Ok(None)
    }
}
