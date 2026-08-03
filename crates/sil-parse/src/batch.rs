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

    if doc.kind == SourceKind::Pdf
        && let Ok(rt) = tokio::runtime::Runtime::new()
        && let Ok(meta) = rt.block_on(crate::xberg_metadata::extract_metadata_utf8(path))
    {
        if !meta.title.trim().is_empty() {
            doc.title = Some(meta.title);
        }
        if !meta.authors.is_empty() {
            doc.authors = Some(meta.authors.join(", "));
        }
        if !meta.citations.is_empty() {
            doc.references_text = Some(meta.citations.join("\n"));
        }
    }

    if doc.references_text.is_none() {
        doc.references_text = crate::references::extract_references_block(&content);
    }

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
    let null_ui = sil_core::NullUi::new();
    for (i, path) in paths.iter().enumerate() {
        pb.set_message(path.file_name().unwrap_or(path.as_str()));
        match parse_one(path, db, runner, &null_ui) {
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
    let mut header_lines = Vec::new();
    for line in content.lines() {
        let clean = sil_regex::strip_html_spans(line).trim().to_string();
        let lower = clean.to_lowercase();
        let stripped_lower = lower.trim_start_matches('#').trim();

        if sil_regex::is_reference_heading(&clean)
            || stripped_lower == "abstract"
            || stripped_lower == "a b s t r a c t"
            || stripped_lower.starts_with("abstract")
            || stripped_lower == "a r t i c l e i n f o"
            || stripped_lower.starts_with("1. introduction")
            || stripped_lower.starts_with("1 introduction")
            || stripped_lower.starts_with("i. introduction")
        {
            break;
        }
        header_lines.push(line.to_string());
        if header_lines.len() >= 60 {
            break;
        }
    }
    let header_text = header_lines.join("\n");

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
    let mut selected_title_idx = None;
    if doc.kind == SourceKind::Text {
        if doc.title.is_none() {
            doc.title = path.file_stem().map(|s| s.to_string());
        }
    } else if doc.title.is_none()
        || doc
            .title
            .as_deref()
            .is_some_and(|t| t.starts_with("page-") || t.len() < 4)
    {
        let mut candidates = Vec::new();
        for (idx, line) in header_lines.iter().enumerate() {
            if sil_regex::is_journal_or_publisher_title(line) {
                continue;
            }
            let clean = sil_regex::strip_html_spans(line).trim().to_string();
            let raw_cand = clean.trim_start_matches('#').trim();
            let cand = raw_cand.trim_matches('*').trim_matches('_').trim();
            let lower = cand.to_lowercase();

            if cand.is_empty() || cand.starts_with("page-") || cand.starts_with("Parsed from") {
                continue;
            }
            if lower == "abstract"
                || lower == "a b s t r a c t"
                || lower == "contents"
                || lower.starts_with("1 introduction")
                || lower.starts_with("1. introduction")
                || lower.starts_with("i. introduction")
            {
                continue;
            }

            let is_h1 = line.trim_start().starts_with("# ");
            let is_any_h = line.trim_start().starts_with('#');
            candidates.push((idx, cand.to_string(), is_h1, is_any_h));
        }

        let chosen = candidates
            .iter()
            .find(|(_, _, is_h1, _)| *is_h1)
            .or_else(|| candidates.iter().find(|(_, _, _, is_h)| *is_h))
            .or_else(|| candidates.first());

        if let Some((idx, title_str, _, _)) = chosen {
            doc.title = Some(title_str.clone());
            selected_title_idx = Some(*idx);
        } else {
            doc.title = path.file_stem().map(|s| s.to_string());
        }
    }

    // Local extraction for Authors
    if doc.authors.is_none() || doc.authors.as_deref().is_some_and(|a| a.trim().is_empty()) {
        let title_line_idx = selected_title_idx.or_else(|| {
            doc.title.as_ref().and_then(|t| {
                let t_clean = t.trim().to_lowercase();
                header_lines.iter().position(|l| {
                    let clean = sil_regex::strip_html_spans(l).trim().trim_start_matches('#').trim().to_lowercase();
                    !t_clean.is_empty() && (clean == t_clean || clean.contains(&t_clean))
                })
            })
        });
        let start_idx = title_line_idx.map_or(0, |i| i + 1);
        let byline_lines = if start_idx < header_lines.len() {
            &header_lines[start_idx..]
        } else {
            &header_lines[..]
        };

        let is_anonymous = byline_lines.iter().any(|line| {
            let l = line.to_lowercase();
            l.contains("anonymous authors") || l.contains("paper under double-blind review")
        });

        if is_anonymous {
            doc.authors = Some("Anonymous authors".to_string());
        } else {
            let mut author_names = Vec::new();
            for line in byline_lines {
                let clean = sil_regex::strip_html_spans(line).trim().to_string();
                let lower = clean.to_lowercase();
                let stripped_lower = lower.trim_start_matches('#').trim();

                if stripped_lower == "abstract"
                    || stripped_lower == "a b s t r a c t"
                    || stripped_lower.starts_with("abstract")
                    || stripped_lower.starts_with("1 introduction")
                    || stripped_lower.starts_with("1. introduction")
                    || stripped_lower.starts_with("keywords")
                    || stripped_lower.starts_with("index terms")
                    || stripped_lower.starts_with("date:")
                    || stripped_lower.starts_with("code:")
                    || stripped_lower.starts_with("data:")
                {
                    break;
                }

                let cleaned_line = sil_regex::clean_author_byline_line(&clean);
                if cleaned_line.is_empty() {
                    continue;
                }

                for name in sil_regex::split_author_names(&cleaned_line) {
                    if is_valid_author_name(&name) && !author_names.contains(&name) {
                        author_names.push(name);
                    }
                }
            }

            if !author_names.is_empty() {
                doc.authors = Some(author_names.join(", "));
            }
        }
    }

    // Local extraction for Year (document header level only)
    if doc.year.is_none() {
        for line in content.lines().take(80) {
            if let Some(y) = sil_regex::extract_header_year(line) {
                doc.year = Some(y);
                break;
            }
        }
    }

    // Local extraction for Venue
    if doc.venue.is_none() {
        for line in &header_lines {
            if let Some(v) = sil_regex::extract_reference_venue(line) {
                doc.venue = Some(v);
                break;
            }
        }
    }
}

fn is_valid_author_name(name: &str) -> bool {
    let t = name.trim();
    if t.is_empty() || t.len() < 2 || t.len() > 60 {
        return false;
    }
    let lower = t.to_lowercase();

    let bad_words = [
        "university",
        "universidad",
        "department",
        "departamento",
        "institute",
        "instituto",
        "school",
        "researcher",
        "abstract",
        "introduction",
        "keywords",
        "date:",
        "correspondence",
        "equal contribution",
        "inc.",
        "github",
        "ieee",
        "senior",
        "member",
        "orcid",
        "email",
        "http",
        "https",
        "january",
        "february",
        "march",
        "april",
        "may",
        "june",
        "some",
        "another",
        "address",
        "singapore",
        "china",
        "japan",
        "research",
        "center",
        "group",
        "lab",
        "joint",
        "alibaba",
        "baidu",
        "tencent",
        "google",
        "meta",
        "amazon",
    ];
    if bad_words.iter().any(|w| lower == *w || lower.contains(w)) {
        return false;
    }

    let first_char = match t.chars().next() {
        Some(c) => c,
        None => return false,
    };
    if !first_char.is_uppercase() {
        return false;
    }

    let words: Vec<&str> = t.split_whitespace().collect();
    if words.is_empty() || words.len() > 5 {
        return false;
    }

    let capitalized_count = words
        .iter()
        .filter(|w| {
            w.chars().next().is_some_and(|c| c.is_uppercase())
                || *w == &"de"
                || *w == &"van"
                || *w == &"von"
                || *w == &"der"
        })
        .count();

    capitalized_count >= (words.len() + 1) / 2
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
    fn test_hydrate_elsevier_journal_header_title_skipping() {
        let mut doc = SourceDocument::new(Utf8PathBuf::from("test3.pdf"));
        doc.kind = SourceKind::Markdown;

        let content = r#"ScienceDirect
# Knowledge-Based Systems
journal homepage: www.elsevier.com/locate/knosys
# Optimising retrieval performance in RAG systems: A new growing window semantic chunking strategy to address weak semantic boundaries
Antonio Moreno-Cediel , Eva Garcia-Lopez , Antonio Garcia-Cabot * , David De-Fitero-Dominguez
*Departamento de Ciencias de la Computacion, Universidad de Alcala, Madrid, Spain
## Abstract"#;

        hydrate_source_document_metadata(&mut doc, content, Utf8Path::new("test3.pdf"));

        assert_eq!(
            doc.title.unwrap(),
            "Optimising retrieval performance in RAG systems: A new growing window semantic chunking strategy to address weak semantic boundaries"
        );
        assert_eq!(
            doc.authors.unwrap(),
            "Antonio Moreno-Cediel, Eva Garcia-Lopez, Antonio Garcia-Cabot, David De-Fitero-Dominguez"
        );
    }

    #[test]
    fn test_hydrate_double_blind_anonymous_authors() {
        let mut doc = SourceDocument::new(Utf8PathBuf::from("test4.pdf"));
        doc.kind = SourceKind::Markdown;

        let content = r#"Paper under double-blind review
# ON THE ENTROPY CALIBRATION OF LANGUAGE MODELS
Anonymous authors
## Abstract"#;

        hydrate_source_document_metadata(&mut doc, content, Utf8Path::new("test4.pdf"));

        assert_eq!(
            doc.title.unwrap(),
            "ON THE ENTROPY CALIBRATION OF LANGUAGE MODELS"
        );
        assert_eq!(doc.authors.unwrap(), "Anonymous authors");
    }

    #[test]
    fn test_hydrate_single_author_independent_researcher() {
        let mut doc = SourceDocument::new(Utf8PathBuf::from("test5.pdf"));
        doc.kind = SourceKind::Markdown;

        let content = r#"# Self-Anchoring Calibration Drift in Large Language Models: How Multi-Turn Conversations Reshape Model Confidence
Harshavardhan Independent Researcher harsh@link.cuhk.edu.hk
May 2026
## Abstract"#;

        hydrate_source_document_metadata(&mut doc, content, Utf8Path::new("test5.pdf"));

        assert_eq!(
            doc.title.unwrap(),
            "Self-Anchoring Calibration Drift in Large Language Models: How Multi-Turn Conversations Reshape Model Confidence"
        );
        assert_eq!(doc.authors.unwrap(), "Harshavardhan");
        assert_eq!(doc.year, Some(2026));
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
