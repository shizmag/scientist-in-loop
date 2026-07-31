//! Extract, clean, and split reference sections into structured ReferenceEntry items.

use sil_core::{ReferenceEntry, SourceId};
use sil_regex::{
    extract_doi, extract_quoted_title, extract_year, is_non_ref_heading, is_reference_entry_start,
    is_reference_heading,
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
        } else if in_refs && is_non_ref_heading(t) {
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
        let (authors, year, title, doi) = parse_entry_metadata(&raw_text);
        let id = format!("{}_ref_{}", source_id.as_str(), idx + 1);

        results.push(ReferenceEntry {
            id,
            source_id: source_id.clone(),
            ref_index: idx + 1,
            raw_text,
            title,
            authors,
            year,
            doi,
        });
    }

    results
}

/// Clean HTML span tags from a line or string.
fn clean_spans(text: &str) -> String {
    sil_regex::strip_html_spans(text).trim().to_string()
}

/// Split a raw reference block into individual citation strings.
fn split_raw_entries(block: &str) -> Vec<String> {
    let raw_lines: Vec<String> = block
        .lines()
        .map(|l| clean_spans(l))
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty() && l != "-" && !is_noise_line(l))
        .collect();

    if raw_lines.is_empty() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let mut current = String::new();

    for line in raw_lines {
        if is_reference_entry_start(&line) {
            if !current.is_empty() {
                entries.push(current.trim().to_string());
                current.clear();
            }
            current.push_str(&line);
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(&line);
        }
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

/// Extract metadata fields (authors, year, title, doi) from a raw citation string.
fn parse_entry_metadata(text: &str) -> (Option<String>, Option<i32>, Option<String>, Option<String>) {
    let doi = extract_doi(text);
    let year = extract_year(text);
    let title = extract_quoted_title(text).or_else(|| extract_unquoted_title(text));
    let authors = extract_authors(text, year, title.as_deref());

    (authors, year, title, doi)
}

/// Try to extract an unquoted title from academic citation format (`[N] Authors. Title. Venue, Year`).
fn extract_unquoted_title(text: &str) -> Option<String> {
    let clean = text.trim();
    // Strip leading list prefix like "- ", "[1] ", "1. "
    let content = if let Some(idx) = clean.find(']') {
        clean[idx + 1..].trim()
    } else if let Some(pos) = clean.find(". ") {
        let first = &clean[..pos];
        if first.chars().all(|c| c.is_ascii_digit()) {
            clean[pos + 2..].trim()
        } else {
            clean
        }
    } else {
        clean.trim_start_matches('-').trim()
    };

    let parts: Vec<&str> = content.split(". ").collect();
    if parts.len() >= 2 {
        let candidate = parts[1].trim().trim_end_matches('.');
        if candidate.len() >= 5 && candidate.len() <= 200 && !candidate.contains("http") && !candidate.contains("doi:") {
            return Some(candidate.to_string());
        }
    } else if parts.len() == 1 {
        let candidate = parts[0].trim().trim_end_matches('.');
        if candidate.len() >= 5 && candidate.len() <= 200 && !candidate.contains("http") && !candidate.contains("doi:") {
            return Some(candidate.to_string());
        }
    }

    None
}

fn extract_authors(text: &str, year: Option<i32>, title: Option<&str>) -> Option<String> {
    let clean = text.trim();

    if let Some(t) = title
        && let Some(pos) = clean.find(t)
    {
        let candidate = clean[..pos]
            .trim_start_matches('-')
            .trim();
        // Strip leading [N] or N.
        let candidate = if let Some(idx) = candidate.find(']') {
            candidate[idx + 1..].trim()
        } else {
            candidate
        };
        let candidate = candidate.trim_end_matches(&[' ', '"', '“', ',', '.'][..]).trim();
        if !candidate.is_empty() && candidate.len() < 150 {
            return Some(candidate.to_string());
        }
    }

    if let Some(y) = year {
        let year_str = y.to_string();
        if let Some(pos) = clean.find(&year_str) {
            let candidate = clean[..pos]
                .trim_start_matches('-')
                .trim();
            let candidate = if let Some(idx) = candidate.find(']') {
                candidate[idx + 1..].trim()
            } else {
                candidate
            };
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
        assert_eq!(entries[0].title.as_deref(), Some("Retrieval-augmented generation for knowledge-intensive nlp tasks"));
        assert_eq!(entries[1].ref_index, 2);
        assert_eq!(entries[1].year, Some(2024));
        assert_eq!(entries[1].title.as_deref(), Some("Benchmarking large language models in retrievalaugmented generation"));
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
        assert_eq!(entries[0].title.as_deref(), Some("Attention is all you need."));
        assert!(entries[0].doi.as_ref().unwrap().contains("10.5555"));
        assert_eq!(entries[1].year, Some(2019));
        assert_eq!(
            entries[1].title.as_deref(),
            Some("BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding.")
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
        assert_eq!(entries[1].title.as_deref(), Some("Computing Machinery and Intelligence."));
    }

    #[test]
    fn test_doi_extraction_variations() {
        let text1 = "[1] Smith et al. 2020. doi:10.1038/s41586-020-1234-y";
        let text2 = "[2] Jones et al. 2021. https://doi.org/10.1016/j.cell.2021.01.001";

        assert_eq!(extract_doi(text1).as_deref(), Some("10.1038/s41586-020-1234-y"));
        assert_eq!(extract_doi(text2).as_deref(), Some("10.1016/j.cell.2021.01.001"));
    }
}
