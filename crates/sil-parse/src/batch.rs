//! Single- and multi-PDF parse orchestration.

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{DocumentStatus, SilUi, SourceDocument, SourceKind};
use sil_db::SilDb;

use crate::error::ParseError;
use crate::marker::MarkerRunner;
use crate::validate::validate_for_parse;

/// Result of parsing one source document.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// Source document metadata.
    pub document: SourceDocument,
    /// Extracted plain text / markdown.
    pub content: String,
    /// Time taken for extraction and processing.
    pub duration: std::time::Duration,
    /// Total reference entries extracted.
    pub reference_count: usize,
}

/// Parse one source document and write into the database.
pub fn parse_one(
    path: &Utf8Path,
    db: &SilDb,
    runner: &dyn MarkerRunner,
    ui: &dyn SilUi,
) -> Result<ParseResult, ParseError> {
    let start_time = std::time::Instant::now();
    let (status, mut doc) = validate_for_parse(path, db)?;
    match status {
        DocumentStatus::Valid(kind) => {
            doc.kind = kind;
        }
        DocumentStatus::NotFound => {
            return Err(ParseError::InvalidDocument(format!(
                "file not found: {path}"
            )));
        }
        DocumentStatus::NotPdf => {
            return Err(ParseError::InvalidDocument(format!(
                "not a PDF file: {path}"
            )));
        }
        DocumentStatus::UnsupportedFormat => {
            return Err(ParseError::InvalidDocument(format!(
                "unsupported format: {path}"
            )));
        }
        DocumentStatus::Corrupted => {
            return Err(ParseError::InvalidDocument(format!(
                "corrupted or unreadable document: {path}"
            )));
        }
        DocumentStatus::AlreadyParsed => {
            return Err(ParseError::InvalidDocument(format!(
                "already parsed: {} (remove DB row to re-parse)",
                doc.filename
            )));
        }
    }

    let mut spin = ui.spinner(&format!("Extracting text: {}", doc.filename));
    let content = match doc.kind {
        SourceKind::Pdf => match runner.parse_pdf(path) {
            Ok(c) => {
                spin.finish_success(&format!("Extracted {}", doc.filename));
                c
            }
            Err(e) => {
                spin.finish_error(&format!("Failed extracting {}", doc.filename));
                return Err(e);
            }
        },
        _ => match std::fs::read_to_string(path) {
            Ok(c) => {
                spin.finish_success(&format!("Read {}", doc.filename));
                c
            }
            Err(e) => {
                spin.finish_error(&format!("Failed reading {}", doc.filename));
                return Err(ParseError::InvalidDocument(format!(
                    "failed to read text content of {path}: {e}"
                )));
            }
        },
    };

    hydrate_source_document_metadata(&mut doc, &content, path);

    doc.references_text = crate::references::extract_references_block(&content);
    doc.parsed = true;
    doc.status = Some(DocumentStatus::Valid(doc.kind));

    db.upsert_parsed(&doc, &content)
        .map_err(|e| ParseError::Db(e.to_string()))?;

    let mut reference_count = 0;
    if let Some(ref raw_block) = doc.references_text {
        let entries = crate::references::parse_reference_entries(&doc.id, raw_block);
        reference_count = entries.len();
        if !entries.is_empty() {
            db.save_source_references(&entries)
                .map_err(|e| ParseError::Db(e.to_string()))?;
        }
    }

    let duration = start_time.elapsed();

    Ok(ParseResult {
        document: doc,
        content,
        duration,
        reference_count,
    })
}

/// Parse many PDFs with a progress bar; returns (ok, failed) counts and errors.
pub fn parse_many(
    paths: &[Utf8PathBuf],
    db: &SilDb,
    runner: &dyn MarkerRunner,
    ui: &dyn SilUi,
) -> (usize, usize, Vec<(Utf8PathBuf, String)>) {
    let total = paths.len() as u64;
    let mut pb = ui.progress(total, "Parsing PDFs");
    let mut ok = 0usize;
    let mut failed = 0usize;
    let mut errors = Vec::new();
    for (i, path) in paths.iter().enumerate() {
        pb.set_message(path.file_name().unwrap_or(path.as_str()));
        match parse_one(path, db, runner, ui) {
            Ok(_) => ok += 1,
            Err(e) => {
                failed += 1;
                errors.push((path.clone(), e.to_string()));
            }
        }
        pb.set_position((i as u64) + 1);
    }
    if failed == 0 {
        pb.finish_success(&format!("Parsed {ok} PDF(s)"));
    } else {
        pb.finish_error(&format!("Parsed {ok} PDF(s), {failed} failed"));
    }
    (ok, failed, errors)
}

/// Hydrate a `SourceDocument`'s metadata (title, authors, year, venue, abstract, doi)
/// using header-scoped DOI/arXiv API lookup and robust frontmatter text parsing.
pub fn hydrate_source_document_metadata(doc: &mut SourceDocument, content: &str, path: &Utf8Path) {
    let header_text: String = if let Some(pos) = content
        .lines()
        .position(|l| sil_regex::is_reference_heading(l.trim()))
    {
        content.lines().take(pos).collect::<Vec<_>>().join("\n")
    } else {
        content.chars().take(4000).collect()
    };

    let header_doi = sil_regex::extract_doi(&header_text);
    let header_arxiv = sil_regex::extract_arxiv_id(&header_text);

    let mut hydrated_by_api = false;

    if let Some(ref doi) = header_doi
        && let Ok(Some(pub_item)) = crate::journal_digest::fetch_work_by_doi(doi)
    {
        if doc.doi.is_none() {
            doc.doi = pub_item.doi.or_else(|| header_doi.clone());
        }
        if doc.title.is_none() && !pub_item.title.is_empty() {
            doc.title = Some(pub_item.title);
        }
        if doc.authors.is_none() && !pub_item.authors.is_empty() {
            doc.authors = Some(pub_item.authors);
        }
        if doc.year.is_none() && pub_item.year.is_some() {
            doc.year = pub_item.year.map(|y| y as i32);
        }
        if doc.venue.is_none() && !pub_item.journal.is_empty() {
            doc.venue = Some(pub_item.journal);
        }
        if doc.abstract_text.is_none() && !pub_item.abstract_text.is_empty() {
            doc.abstract_text = Some(pub_item.abstract_text);
        }
        hydrated_by_api = true;
    }

    if !hydrated_by_api
        && let Some(ref arxiv) = header_arxiv
        && let Ok(Some(pub_item)) = crate::journal_digest::fetch_work_by_arxiv_id(arxiv)
    {
        if doc.doi.is_none() {
            doc.doi = pub_item.doi;
        }
        if doc.title.is_none() && !pub_item.title.is_empty() {
            doc.title = Some(pub_item.title);
        }
        if doc.authors.is_none() && !pub_item.authors.is_empty() {
            doc.authors = Some(pub_item.authors);
        }
        if doc.year.is_none() && pub_item.year.is_some() {
            doc.year = pub_item.year.map(|y| y as i32);
        }
        if doc.venue.is_none() && !pub_item.journal.is_empty() {
            doc.venue = Some(pub_item.journal);
        }
        if doc.abstract_text.is_none() && !pub_item.abstract_text.is_empty() {
            doc.abstract_text = Some(pub_item.abstract_text);
        }
    }

    if doc.doi.is_none() {
        doc.doi = header_doi;
    }

    // Local extraction for Title
    if doc.title.is_none()
        || doc
            .title
            .as_deref()
            .is_some_and(|t| t.starts_with("page-") || t.len() < 4)
    {
        let mut extracted_title = None;
        for line in header_text.lines() {
            let clean = sil_regex::strip_html_spans(line).trim().to_string();
            if clean.starts_with('#') {
                let candidate = clean
                    .trim_start_matches('#')
                    .trim()
                    .trim_matches('*')
                    .trim();
                let lower = candidate.to_lowercase();
                if !candidate.is_empty()
                    && !candidate.starts_with("page-")
                    && lower != "abstract"
                    && lower != "contents"
                    && !lower.starts_with("1 introduction")
                    && !lower.starts_with("contents lists")
                    && !lower.starts_with("journal homepage")
                    && lower != "knowledge-based systems"
                    && lower != "sciencedirect"
                    && !lower.contains("elsevier")
                {
                    extracted_title = Some(candidate.to_string());
                    break;
                }
            }
        }
        if extracted_title.is_none() {
            extracted_title = path.file_stem().map(|s| s.to_string());
        }
        doc.title = extracted_title;
    }

    // Local extraction for Authors
    if doc.authors.is_none() || doc.authors.as_deref().is_some_and(|a| a.trim().is_empty()) {
        let mut author_lines = Vec::new();
        let mut past_title = false;
        for (i, line) in header_text.lines().enumerate() {
            let clean = sil_regex::strip_html_spans(line).trim().to_string();
            let lower_clean = clean.to_lowercase();

            if lower_clean == "abstract" 
                || lower_clean.starts_with("1 introduction") 
                || lower_clean.starts_with("keywords") 
                || lower_clean.starts_with("index terms") 
                || lower_clean.contains("date:") 
                || lower_clean.contains("code:") 
                || lower_clean.contains("data:") 
            {
                break;
            }

            let is_heading = clean.starts_with('#');
            let is_title_match = doc.title.as_deref().is_some_and(|t| clean.contains(t) || t.contains(&clean));

            if !past_title
                && (is_heading || is_title_match || i == 0) {
                    past_title = true;
                    continue;
                }

            if past_title && !clean.is_empty() {
                if is_heading {
                    break;
                }
                if sil_regex::is_affiliation_or_noise_line(&clean) {
                    continue;
                }
                let lower = clean.to_lowercase();
                if lower.starts_with("january")
                    || lower.starts_with("february")
                    || lower.starts_with("march")
                    || lower.starts_with("april")
                    || lower.starts_with("may")
                    || lower.starts_with("june")
                    || lower.starts_with("july")
                    || lower.starts_with("august")
                    || lower.starts_with("september")
                    || lower.starts_with("october")
                    || lower.starts_with("november")
                    || lower.starts_with("december")
                {
                    continue;
                }

                let cleaned_author = sil_regex::strip_markdown_links(&clean);
                let cleaned_author = sil_regex::strip_author_footnote_markers(&cleaned_author);

                let cleaned_author = cleaned_author
                    .replace(['*', '⋈', '†', '‡', '§', '¶', '♯', '♠', '¹', '²', '³', '⁴', '⁵', '⁶', '⁷', 'ⁿ', '՞', 'ã', 'ゥ', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9'], "")
                    .trim_start_matches('-')
                    .trim_matches(|c: char| c.is_ascii_punctuation() || c.is_whitespace())
                    .to_string();

                // Collapse multiple spaces and trim commas
                let cleaned_author = cleaned_author
                    .split_whitespace()
                    .collect::<Vec<_>>()
                    .join(" ")
                    .replace(" ,", ",")
                    .trim_matches(|c: char| c == ',' || c.is_whitespace())
                    .to_string();

                if !cleaned_author.is_empty() && cleaned_author.len() < 150 {
                    // Reject lines that start with non-author lower-case text
                    if cleaned_author.chars().next().is_some_and(|c| c.is_lowercase()) {
                        continue;
                    }

                    // Validate capitalized words ratio
                    let words: Vec<&str> = cleaned_author.split_whitespace().collect();
                    let word_count = words.len();
                    if word_count > 0 {
                        let capitalized_count = words.iter().filter(|w| w.chars().next().is_some_and(|c| c.is_uppercase())).count();
                        if word_count > 15 || (word_count > 3 && capitalized_count < word_count / 2) {
                            continue;
                        }
                    }

                    author_lines.push(cleaned_author);
                    // Extracting up to a few lines to avoid getting into noise,
                    // but we can increase if there are many authors.
                    if author_lines.len() >= 3 {
                        break;
                    }
                }
            }
        }
        if !author_lines.is_empty() {
            doc.authors = Some(author_lines.join(", "));
        }
    }

    // Local extraction for Year
    if doc.year.is_none() {
        for line in header_text.lines().take(20) {
            if let Some(y) = sil_regex::extract_year(line) {
                doc.year = Some(y);
                break;
            }
        }
    }

    // Local extraction for Venue
    if doc.venue.is_none() {
        for line in header_text.lines().take(25) {
            if let Some(v) = sil_regex::extract_reference_venue(line) {
                doc.venue = Some(v);
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use camino::{Utf8Path, Utf8PathBuf};
    use sil_core::{SourceDocument, SourceKind};

    #[test]
    fn test_hydrate_author_metadata_clean() {
        let mut doc = SourceDocument::new(Utf8PathBuf::from("test.pdf"));
        doc.kind = SourceKind::Pdf;
        doc.title = Some("Test Title".to_string());
        
        let header = r#"# Test Title
[Sebastian Farquhar](#page-1-0)1, [Jannik Kossen](#page-1-0)1 2, [Lorenz Kuhn](#page-1-0)1, [Yarin Gal](#page-1-0)1
1 University of Oxford 2 OATML
Abstract"#;

        hydrate_source_document_metadata(&mut doc, header, Utf8Path::new("test.pdf"));
        
        assert_eq!(
            doc.authors.unwrap(),
            "Sebastian Farquhar, Jannik Kossen, Lorenz Kuhn, Yarin Gal"
        );
    }

    #[test]
    fn test_hydrate_author_with_footnote_noise() {
        let mut doc = SourceDocument::new(Utf8PathBuf::from("test2.pdf"));
        doc.kind = SourceKind::Pdf;
        doc.title = Some("Another Paper".to_string());
        
        let header = r#"# Another Paper
Ushtar Ali<sup>a</sup>, Steven Lynden<sup>b</sup>, Akiyoshi Matono<sup>b</sup>
<sup>a</sup> Some University Address
<sup>b</sup> Another Department Address
1 Introduction"#;

        hydrate_source_document_metadata(&mut doc, header, Utf8Path::new("test2.pdf"));
        
        assert_eq!(
            doc.authors.unwrap(),
            "Ushtar Ali, Steven Lynden, Akiyoshi Matono"
        );
    }

    #[test]
    fn test_hydrate_plain_text_authors() {
        let mut doc = SourceDocument::new(Utf8PathBuf::from("2026.gem-main.4.pdf"));
        doc.kind = SourceKind::Pdf;
        doc.title = Some("Implicit Ensembles of Ensem".to_string());
        
        let header = r#"Implicit Ensembles of Ensem
Sebastian Farquhar, Armen Der Kiureghian
a Alibaba-NTU Singapore Joint Research Institute
journal homepage:
Keywords: something
Abstract"#;

        hydrate_source_document_metadata(&mut doc, header, Utf8Path::new("2026.gem-main.4.pdf"));
        
        assert_eq!(
            doc.authors.unwrap(),
            "Sebastian Farquhar, Armen Der Kiureghian"
        );
    }
}
