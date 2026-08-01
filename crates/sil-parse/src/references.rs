//! Extract, clean, and split reference sections into structured ReferenceEntry items.

use sil_core::{ReferenceEntry, SourceId};
use sil_regex::{
    extract_doi, extract_quoted_title, extract_year, is_non_ref_heading, is_reference_heading,
};

/// Extract raw reference text block from parsed Markdown content.
pub fn extract_references_block(content: &str) -> Option<String> {
    let mut references = Vec::new();
    let mut in_refs = false;

    for line in content.lines() {
        let t = line.trim();
        if is_reference_heading(t) {
            in_refs = true;
            continue;
        } else if in_refs && (is_non_ref_heading(t) || sil_regex::is_biography_or_prose_line(t)) {
            break;
        }

        if in_refs {
            references.push(line);
        }
    }

    let joined = references.join("\n").trim().to_string();
    if joined.is_empty() {
        None
    } else {
        Some(joined)
    }
}

/// Split raw reference block into individual structured ReferenceEntry items.
pub fn parse_reference_entries(source_id: &SourceId, raw_block: &str) -> Vec<ReferenceEntry> {
    let raw_entries = split_raw_entries(raw_block);
    let mut results = Vec::new();

    for (idx, raw_text) in raw_entries.into_iter().enumerate() {
        let (authors, year, title, venue, doi) = parse_entry_metadata(&raw_text);
        let id = format!("{}_ref_{}", source_id.as_str(), idx + 1);

        results.push(ReferenceEntry {
            id,
            source_id: source_id.clone(),
            ref_index: idx + 1,
            raw_text,
            title,
            authors,
            year,
            venue,
            doi,
        });
    }

    results
}

/// Clean HTML span tags from a line or string.
fn clean_spans(text: &str) -> String {
    sil_regex::strip_html_spans(text).trim().to_string()
}

/// Detected numbering format used in a reference list.
#[derive(Debug, Clone, Copy)]
enum RefNumberFormat {
    /// `[1]`, `[2]`, ...
    Bracketed,
    /// `(1)`, `(2)`, ...
    Parenthesized,
    /// `1.`, `2.`, ...
    DotNumbered,
}

impl RefNumberFormat {
    /// Build the expected marker string for index `n`.
    fn marker(&self, n: usize) -> String {
        match self {
            Self::Bracketed => format!("[{n}]"),
            Self::Parenthesized => format!("({n})"),
            Self::DotNumbered => format!("{n}."),
        }
    }
}

/// Try to detect the numbering format from a cleaned line.
fn detect_number_format(line: &str) -> Option<(RefNumberFormat, usize)> {
    let t = line.trim_start_matches('-').trim();
    // [N]
    if t.starts_with('[') {
        if let Some(end) = t.find(']') {
            if let Ok(n) = t[1..end].parse::<usize>() {
                return Some((RefNumberFormat::Bracketed, n));
            }
        }
    }
    // (N) — but only small numbers to avoid matching "(2024)" year patterns
    if t.starts_with('(') {
        if let Some(end) = t.find(')') {
            if let Ok(n) = t[1..end].parse::<usize>() {
                if n < 500 {
                    // Check that after ')' there is a space and then text (not a year-like pattern)
                    let rest = t[end + 1..].trim_start();
                    if !rest.is_empty() && rest.starts_with(|c: char| c.is_alphabetic()) {
                        return Some((RefNumberFormat::Parenthesized, n));
                    }
                }
            }
        }
    }
    // N. (only at start, followed by space and alphabetic)
    if let Some(dot_pos) = t.find(". ") {
        let prefix = &t[..dot_pos];
        if let Ok(n) = prefix.parse::<usize>() {
            if n < 500 {
                return Some((RefNumberFormat::DotNumbered, n));
            }
        }
    }
    None
}

/// Check if a cleaned line starts with the expected marker for number `n` in the given format.
fn line_starts_with_marker(line: &str, fmt: RefNumberFormat, n: usize) -> bool {
    let marker = fmt.marker(n);
    let t = line.trim_start_matches('-').trim();
    t.starts_with(&marker)
}

/// Split a raw reference block into individual citation strings.
///
/// Two strategies:
/// 1. **Numbered**: detect `[1]`, `(1)`, or `1.` → split by sequential markers.
/// 2. **Unnumbered**: split by blank lines — each paragraph is one reference.
fn split_raw_entries(block: &str) -> Vec<String> {
    // Phase 1: detect numbering format by scanning cleaned lines
    let mut detected: Option<(RefNumberFormat, usize)> = None;
    for raw_line in block.lines() {
        let cleaned = clean_spans(raw_line);
        let trimmed = cleaned.trim();
        if trimmed.is_empty() || is_noise_line(trimmed) {
            continue;
        }
        if let Some(d) = detect_number_format(trimmed) {
            detected = Some(d);
            break;
        }
    }

    // Phase 2: split
    let entries = if let Some((fmt, start_n)) = detected {
        let lines: Vec<String> = block
            .lines()
            .map(clean_spans)
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty() && l != "-" && !is_noise_line(l) && !is_math_line(l))
            .collect();
        split_by_sequential_markers(&lines, fmt, start_n)
    } else {
        split_by_regex_or_paragraphs(block)
    };

    // Phase 3: clean
    let mut cleaned_entries = Vec::new();
    for entry in entries {
        let cleaned = sil_regex::clean_reference_text(&entry);
        if cleaned.is_empty() {
            continue;
        }
        cleaned_entries.push(cleaned);
    }

    cleaned_entries
}

fn split_by_regex_or_paragraphs(block: &str) -> Vec<String> {
    let lines: Vec<String> = block
        .lines()
        .map(clean_spans)
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && l != "-" && !is_noise_line(l) && !is_math_line(l))
        .collect();

    let matches_count = lines
        .iter()
        .filter(|l| sil_regex::is_reference_entry_start(l))
        .count();

    if matches_count >= 2 {
        let mut entries = Vec::new();
        let mut current = String::new();

        for line in lines {
            if sil_regex::is_reference_entry_start(&line) {
                if !current.is_empty() {
                    entries.push(current.trim().to_string());
                    current.clear();
                }
                current.push_str(&line);
            } else if !current.is_empty() {
                current.push(' ');
                current.push_str(&line);
            }
        }
        if !current.is_empty() {
            entries.push(current.trim().to_string());
        }
        entries
    } else {
        split_by_paragraphs(block)
    }
}

/// Check if a line is LaTeX math noise.
fn is_math_line(line: &str) -> bool {
    line.contains("$$")
        || line.contains("\\mid")
        || line.contains("\\mathbf")
        || line.contains("\\mathcal")
}

/// Split lines by sequential numbered markers (`[1]`, `[2]`, ... or `1.`, `2.`, ...).
/// Stops when the next expected number is not found within a reasonable lookahead.
fn split_by_sequential_markers(
    lines: &[String],
    fmt: RefNumberFormat,
    start_n: usize,
) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = String::new();
    let mut next_expected = start_n;

    for line in lines {
        if line_starts_with_marker(line, fmt, next_expected) {
            if !current.is_empty() {
                entries.push(current.trim().to_string());
                current.clear();
            }
            current.push_str(line);
            next_expected += 1;
        } else if !current.is_empty() {
            // Continuation line of the current entry
            current.push(' ');
            current.push_str(line);
        }
        // Lines before the first marker are silently skipped
    }

    if !current.is_empty() {
        entries.push(current.trim().to_string());
    }

    entries
}

/// Split by blank lines — each paragraph is one reference entry.
/// For unnumbered reference lists (APA style, bullet-point lists, etc.).
fn split_by_paragraphs(block: &str) -> Vec<String> {
    let mut entries = Vec::new();
    let mut current = String::new();

    for raw_line in block.lines() {
        let cleaned = clean_spans(raw_line);
        let trimmed = cleaned.trim();

        if trimmed.is_empty() {
            // Blank line = paragraph boundary
            if !current.is_empty() {
                entries.push(current.trim().to_string());
                current.clear();
            }
            continue;
        }

        if is_noise_line(trimmed) || is_math_line(trimmed) || trimmed == "-" {
            continue;
        }

        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(trimmed);
    }

    if !current.is_empty() {
        entries.push(current.trim().to_string());
    }

    entries
}

/// Check if line is header/footer/page noise.
fn is_noise_line(line: &str) -> bool {
    let lower = line.to_lowercase();
    lower.starts_with("page ") || lower.starts_with("arxiv:") && lower.contains("[cs.")
}

/// Extract metadata fields (authors, year, title, venue, doi) from a raw citation string.
#[allow(clippy::type_complexity)]
fn parse_entry_metadata(
    text: &str,
) -> (
    Option<String>,
    Option<i32>,
    Option<String>,
    Option<String>,
    Option<String>,
) {
    let doi = extract_doi(text);
    let year = extract_year(text);
    let title = extract_quoted_title(text).or_else(|| extract_unquoted_title(text));
    let authors = extract_authors(text, year, title.as_deref());
    let venue = sil_regex::extract_reference_venue(text);

    (authors, year, title, venue, doi)
}

fn extract_unquoted_title(text: &str) -> Option<String> {
    let clean = text.trim();
    let parts: Vec<&str> = clean.split(". ").collect();

    for part in parts {
        let candidate = part.trim().trim_end_matches('.');
        if is_valid_title(candidate) {
            return Some(candidate.to_string());
        }
    }

    None
}

fn is_valid_title(candidate: &str) -> bool {
    let t = candidate.trim();
    if t.len() < 5 || t.len() > 200 || t.contains("http") || t.contains("doi:") {
        return false;
    }
    if t.ends_with("et al") || t.ends_with("et al.") {
        return false;
    }
    if let Some(last_comma) = t.split(',').last() {
        let s = last_comma.trim();
        if s.len() == 1 && s.chars().next().map_or(false, |c| c.is_uppercase()) {
            return false;
        }
    }
    true
}

fn extract_authors(text: &str, year: Option<i32>, title: Option<&str>) -> Option<String> {
    let clean = text.trim();

    if let Some(t) = title
        && let Some(pos) = clean.find(t)
    {
        let candidate = clean[..pos].trim();
        let candidate = candidate
            .trim_end_matches(&[' ', '"', '“', ',', '.'][..])
            .trim();
        if !candidate.is_empty() && candidate.len() < 150 {
            return Some(candidate.to_string());
        }
    }

    if let Some(y) = year {
        let year_str = y.to_string();
        if let Some(pos) = clean.find(&year_str) {
            let candidate = clean[..pos].trim();
            let candidate = candidate
                .trim_end_matches(&[' ', '(', ')', ',', '.'][..])
                .trim();
            if !candidate.is_empty() && candidate.len() < 150 {
                return Some(candidate.to_string());
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_references_block() {
        let md = r#"
# Introduction
Some content...

## References
[1] Vaswani et al. Attention is all you need. NeurIPS 2017.
[2] Devlin et al. BERT. NAACL 2019.
"#;
        let refs = extract_references_block(md).unwrap();
        assert!(refs.contains("Vaswani"));
        assert!(refs.contains("Devlin"));
    }

    #[test]
    fn test_heading_variations_and_appendix_termination() {
        for header in [
            "# Bibliography",
            "## Works Cited",
            "### Literature Cited",
            "# References and Notes",
            "## 10. References",
            "## **References**",
            "# REFERENCES",
        ] {
            let md = format!(
                "# Intro\nText\n\n{header}\n[1] Author A. \"Title A\" 2021.\n\n# Appendix\nAppendix text"
            );
            let refs = extract_references_block(&md).unwrap();
            assert!(refs.contains("Author A"), "Failed for header: {header}");
            assert!(
                !refs.contains("Appendix text"),
                "Failed to terminate on Appendix for header: {header}"
            );
        }
    }

    #[test]
    fn test_parse_span_tagged_entries() {
        let sid = SourceId::new("paper.pdf");
        let raw = r#"
- <span id="page-10-0"></span>[1] Patrick Lewis, Ethan Perez, et al. "Retrieval-augmented generation for knowledge-intensive nlp tasks." NeurIPS 2020.
- <span id="page-10-1"></span>[2] Jiawei Chen, Hongyu Lin, et al. "Benchmarking large language models in retrievalaugmented generation." AAAI 2024.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ref_index, 1);
        assert_eq!(entries[0].year, Some(2020));
        assert_eq!(entries[1].ref_index, 2);
        assert_eq!(entries[1].year, Some(2024));
    }

    #[test]
    fn test_multiline_span_tag_splitting() {
        let sid = SourceId::new("paper.pdf");
        let raw = r#"
- <span id="page-52-4"></span>
[1] Patrick Lewis, Ethan Perez, et al. Retrieval-augmented generation for knowledge-intensive nlp tasks. NeurIPS 2020.
- <span id="page-52-5"></span>
[2] Jiawei Chen, Hongyu Lin, et al. Benchmarking large language models in retrievalaugmented generation. AAAI 2024.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ref_index, 1);
        assert_eq!(entries[0].year, Some(2020));
        assert_eq!(
            entries[0].title.as_deref(),
            Some("Retrieval-augmented generation for knowledge-intensive nlp tasks")
        );
        assert_eq!(entries[1].ref_index, 2);
        assert_eq!(entries[1].year, Some(2024));
        assert_eq!(
            entries[1].title.as_deref(),
            Some("Benchmarking large language models in retrievalaugmented generation")
        );
    }

    #[test]
    fn test_parse_reference_entries() {
        let sid = SourceId::new("paper.pdf");
        let raw = r#"
[1] Vaswani, A., et al. "Attention is all you need." Advances in Neural Information Processing Systems, 2017. doi:10.5555/3295222.3295349
[2] Devlin, J., et al. "BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding." NAACL 2019.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].year, Some(2017));
        assert_eq!(
            entries[0].title.as_deref(),
            Some("Attention is all you need.")
        );
        assert!(entries[0].doi.as_ref().unwrap().contains("10.5555"));
        assert_eq!(entries[1].year, Some(2019));
        assert_eq!(
            entries[1].title.as_deref(),
            Some(
                "BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding."
            )
        );
    }

    #[test]
    fn test_multiline_and_numbered_dot_format() {
        let sid = SourceId::new("doc2.pdf");
        let raw = r#"
1. Shannon, C. E. (1948). A mathematical theory of
communication. Bell System Technical Journal, 27, 379-423.
Page 400
2. Turing, A. M. "Computing Machinery and Intelligence."
Mind, 59, 433-460, 1950.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ref_index, 1);
        assert_eq!(entries[0].year, Some(1948));
        assert!(entries[0].raw_text.contains("communication"));
        // Noise line "Page 400" filtered out
        assert!(!entries[0].raw_text.contains("Page 400"));
        assert!(!entries[1].raw_text.contains("Page 400"));

        assert_eq!(entries[1].ref_index, 2);
        assert_eq!(entries[1].year, Some(1950));
        assert_eq!(
            entries[1].title.as_deref(),
            Some("Computing Machinery and Intelligence.")
        );
    }

    #[test]
    fn test_doi_extraction_variations() {
        let text1 = "[1] Smith et al. 2020. doi:10.1038/s41586-020-1234-y";
        let text2 = "[2] Jones et al. 2021. https://doi.org/10.1016/j.cell.2021.01.001";

        assert_eq!(
            extract_doi(text1).as_deref(),
            Some("10.1038/s41586-020-1234-y")
        );
        assert_eq!(
            extract_doi(text2).as_deref(),
            Some("10.1016/j.cell.2021.01.001")
        );
    }

    #[test]
    fn test_filtering_math_equations() {
        let sid = SourceId::new("math.pdf");
        let raw = r#"
[1] Vaswani et al. 2017. Attention.
$$ I[Y; M] = \sum_x ... $$
[2] Devlin et al. 2019. BERT.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].ref_index, 1);
        assert_eq!(entries[1].ref_index, 2);
        assert!(entries.iter().all(|e| !e.raw_text.contains("$$")));
    }

    #[test]
    fn test_sequential_numbered_entries() {
        let sid = SourceId::new("steps.pdf");
        // Sequential detection: [1], [2] are parsed; anything after the sequence breaks is ignored.
        let raw = r#"
[1] Turing, A. M. (1950). Computing Machinery.
[2] Shannon, C. E. (1948). A mathematical theory of communication.
This is not a reference, it's trailing text from the paper.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].year, Some(1950));
        assert!(entries[0].raw_text.contains("Computing Machinery"));
        assert_eq!(entries[1].year, Some(1948));
    }

    #[test]
    fn test_parsing_marker_span_tagged_references() {
        let sid = SourceId::new("marker.pdf");
        let raw = r#"
- <span id="page-10-0"></span>[1] Vaswani et al. "Attention is all you need." NeurIPS, 2017.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].year, Some(2017));
        assert_eq!(
            entries[0].title.as_deref(),
            Some("Attention is all you need.")
        );
        // The raw_text in entry might still contain some things based on how clean_reference_text works,
        // but it should strip the span tag. Let's verify the span tag is stripped from authors/title parsing.
        assert!(!entries[0].authors.as_deref().unwrap_or("").contains("span"));
    }

    #[test]
    fn test_parsing_apa_citations() {
        let sid = SourceId::new("apa.pdf");
        let raw = r#"
- Farquhar, S., Kossen, J., Kuhn, L., & Gal, Y. (2024). Detecting hallucinations... Nature, 630.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].year, Some(2024));
        assert_eq!(entries[0].venue.as_deref(), Some("Nature"));
    }

    #[test]
    fn test_parsing_elsevier_refhub_links() {
        let sid = SourceId::new("elsevier.pdf");
        let raw = r#"
[2] [X. Guan](#refhub), [Y. Wang](#refhub), via autonomous knowledge graph-based retrofitting, in: Proceedings of the AAAI, 2021.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].year, Some(2021));
        assert_eq!(entries[0].venue.as_deref(), Some("AAAI"));
        // Markdown links to internal anchors like #refhub should be stripped in clean_reference_text,
        // so authors should be clean: "X. Guan, Y. Wang" or similar.
        let cleaned_raw = &entries[0].raw_text;
        assert!(!cleaned_raw.contains("(#refhub)"));
        assert!(cleaned_raw.contains("X. Guan"));
    }

    #[test]
    fn test_is_noise_line() {
        assert!(is_noise_line("Page 42"));
        assert!(is_noise_line("page 1 of 10"));
        assert!(is_noise_line("arXiv:2405.12345v1 [cs.CL]"));
        assert!(!is_noise_line(
            "Vaswani et al. Attention is all you need. 2017."
        ));
    }

    #[test]
    fn test_extract_unquoted_title_and_authors() {
        let text = "Smith, J. Quantum Computing Foundations. Journal of Physics, 2020.";
        let (authors, year, title, venue, doi) = parse_entry_metadata(text);
        assert_eq!(year, Some(2020));
        assert_eq!(title.as_deref(), Some("Quantum Computing Foundations"));
        assert_eq!(authors.as_deref(), Some("Smith, J"));
        assert_eq!(venue.as_deref(), Some("Journal of Physics, 2020"));
        assert_eq!(doi, None);
    }

    #[test]
    fn test_split_raw_entries_math_filtering() {
        let sid = SourceId::new("math.pdf");
        let raw = r#"
[1] Vaswani et al. Attention is all you need. 2017.
[2] Formula equation: $$ E = mc^2 \mid \mathbf{v} \mathcal{M} $$ 2020.
"#;
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].year, Some(2017));
    }

    #[test]
    fn test_extract_unquoted_title_with_author_initials() {
        let text = "J. Kossen, L. Kuhn, Y. Gal. Detecting hallucinations in LLMs. Nature, 2024.";
        let (authors, year, title, venue, doi) = parse_entry_metadata(text);
        assert_eq!(title.as_deref(), Some("Detecting hallucinations in LLMs"));
        assert_eq!(authors.as_deref(), Some("J. Kossen, L. Kuhn, Y. Gal"));
        assert_eq!(year, Some(2024));
        assert_eq!(venue.as_deref(), Some("Nature"));
        assert_eq!(doi, None);
    }

    #[test]
    fn test_split_unnumbered_single_line_entries() {
        let sid = SourceId::new("unnumbered.pdf");
        let raw = "Kossen, J. et al. Title A. 2024.\nKuhn, L. et al. Title B. 2025.";
        let entries = parse_reference_entries(&sid, raw);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].year, Some(2024));
        assert_eq!(entries[1].year, Some(2025));
    }
}
