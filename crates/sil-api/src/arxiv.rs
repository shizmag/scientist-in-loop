use crate::doi::DoiMetadataResult;
use crate::error::ApiError;
use crate::ratelimit::enforce_api_ratelimit;
use crate::retry::with_retry;
use sil_core::JournalPublication;

/// Clean arXiv ID string by stripping prefixes (`arxiv:`, `arXiv:`, URLs) and whitespace.
pub fn clean_arxiv_id_str(arxiv_id: &str) -> String {
    let mut s = arxiv_id.trim();
    if let Some(pos) = s.find("arxiv.org/abs/") {
        s = s[pos + 14..].trim();
    } else if let Some(pos) = s.find("arxiv.org/pdf/") {
        s = s[pos + 14..].trim_end_matches(".pdf").trim();
    }
    if let Some(stripped) = s.strip_prefix("arxiv:") {
        s = stripped.trim();
    } else if let Some(stripped) = s.strip_prefix("arXiv:") {
        s = stripped.trim();
    }
    s.to_string()
}

/// Fetch official BibTeX string directly from arXiv API (`https://arxiv.org/bibtex/{clean_id}`).
pub fn fetch_bibtex_by_arxiv_id(arxiv_id: &str) -> Result<Option<String>, ApiError> {
    let clean_id = clean_arxiv_id_str(arxiv_id);

    if clean_id.is_empty() {
        return Ok(None);
    }

    with_retry(|| {
        enforce_api_ratelimit();
        let url = format!("https://arxiv.org/bibtex/{clean_id}");

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
                    "Rate limited (429) fetching arXiv BibTeX for '{clean_id}'"
                )));
            }
            Err(ureq::Error::Status(s, _)) => {
                return Err(ApiError::NetworkError(format!(
                    "HTTP status {s} fetching arXiv BibTeX for '{clean_id}'"
                )));
            }
            Err(ureq::Error::Transport(e)) => {
                return Err(ApiError::NetworkError(format!(
                    "Network transport error fetching arXiv BibTeX for '{clean_id}': {e}"
                )));
            }
        };

        let body = response
            .into_string()
            .map_err(|e| ApiError::ParseError(format!("Failed to read arXiv BibTeX: {e}")))?;

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

/// Fetch paper metadata by arXiv ID (e.g. `2405.12345` or `arXiv:2405.12345v1`) from arXiv API.
pub fn fetch_work_by_arxiv_id(arxiv_id: &str) -> Result<Option<JournalPublication>, ApiError> {
    let clean_id = clean_arxiv_id_str(arxiv_id);

    if clean_id.is_empty() {
        return Ok(None);
    }

    with_retry(|| {
        enforce_api_ratelimit();
        let url = format!("https://export.arxiv.org/api/query?id_list={clean_id}");
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
                ApiError::RateLimited("arXiv API rate limit exceeded".into())
            }
            ureq::Error::Status(s, _) => ApiError::NetworkError(format!("HTTP status {s}")),
            ureq::Error::Transport(t) => ApiError::NetworkError(format!("ArXiv API request failed: {t}")),
        })?;

    let xml = response
        .into_string()
        .map_err(|e| ApiError::ParseError(format!("Failed to read ArXiv response string: {e}")))?;

    if !xml.contains("<entry>") {
        return Ok(None);
    }

    let entry_start = match xml.find("<entry>") {
        Some(pos) => pos,
        None => return Ok(None),
    };
    let entry_xml = &xml[entry_start..];

    let extract_tag = |tag: &str| -> Option<String> {
        let open = format!("<{tag}>");
        let close = format!("</{tag}>");
        if let Some(sp) = entry_xml.find(&open)
            && let Some(ep) = entry_xml[sp..].find(&close)
        {
            let raw = &entry_xml[sp + open.len()..sp + ep];
            let clean = raw.replace('\n', " ").trim().to_string();
            if !clean.is_empty() {
                return Some(clean);
            }
        }
        None
    };

    let title = extract_tag("title").unwrap_or_default();
    if title.is_empty() || title.contains("Error") {
        return Ok(None);
    }

    let mut authors_vec = Vec::new();
    let mut search_pos = 0;
    while let Some(sp) = entry_xml[search_pos..].find("<author>") {
        let abs_sp = search_pos + sp;
        if let Some(ep) = entry_xml[abs_sp..].find("</author>") {
            let author_block = &entry_xml[abs_sp..abs_sp + ep];
            if let Some(nsp) = author_block.find("<name>")
                && let Some(nep) = author_block[nsp..].find("</name>")
            {
                let name = author_block[nsp + 6..nsp + nep].trim();
                if !name.is_empty() {
                    authors_vec.push(name.to_string());
                }
            }
            search_pos = abs_sp + ep + 9;
        } else {
            break;
        }
    }
    let authors = authors_vec.join(", ");

    let published = extract_tag("published").unwrap_or_default();
    let year = sil_regex::extract_year(&published);
    let abstract_text = extract_tag("summary").unwrap_or_default();
    let doi = extract_tag("arxiv:doi");

        Ok(Some(JournalPublication {
            doi,
            title,
            authors,
            journal: format!("arXiv:{clean_id}"),
            year: year.map(|y| y as u32),
            abstract_text,
            citation_count: None,
            url: format!("https://arxiv.org/abs/{clean_id}"),
            pdf_url: Some(format!("https://arxiv.org/pdf/{clean_id}.pdf")),
        }))
    })
}

/// Verify arXiv ID existence and retrieve official paper title metadata.
///
/// Uses `fetch_work_by_arxiv_id` (which queries `https://export.arxiv.org/api/query?id_list={clean_id}`).
/// Returns `Ok(DoiMetadataResult { exists: true, title: Some(extracted_title) })` if entry exists,
/// `Ok(DoiMetadataResult { exists: false, title: None })` if empty or entry not found,
/// or `Err(ApiError)` on network or rate-limit error.
pub fn verify_arxiv_with_metadata(arxiv_id: &str) -> Result<DoiMetadataResult, ApiError> {
    let clean_id = clean_arxiv_id_str(arxiv_id);
    if clean_id.is_empty() {
        return Ok(DoiMetadataResult {
            exists: false,
            title: None,
        });
    }

    match fetch_work_by_arxiv_id(&clean_id)? {
        Some(work) => {
            let title = if work.title.trim().is_empty() {
                None
            } else {
                Some(work.title)
            };
            Ok(DoiMetadataResult {
                exists: true,
                title,
            })
        }
        None => Ok(DoiMetadataResult {
            exists: false,
            title: None,
        }),
    }
}

