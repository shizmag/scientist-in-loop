//! Extract, clean, and split reference sections into structured ReferenceEntry items.

use sil_core::{ReferenceEntry, SourceId};

/// Extract raw reference text block from parsed Markdown content.
pub fn extract_references_block(content: &str) -> Option<String> {
    let mut references = Vec::new();
    let mut in_refs = false;

    for line in content.lines() {
        let t = line.trim();
        if is_reference_heading(t) {
            in_refs = true;
            continue;
        } else if in_refs && is_major_non_ref_heading(t) {
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

/// Check if line is a reference section heading.
fn is_reference_heading(line: &str) -> bool {
    let clean = line
        .trim_start_matches('#')
        .trim()
        .trim_start_matches(|c: char| c.is_numeric() || c == '.' || c == ' ')
        .trim()
        .to_lowercase();

    matches!(
        clean.as_str(),
        "references"
            | "bibliography"
            | "literature cited"
            | "works cited"
            | "references and notes"
    ) || clean.ends_with(" references")
        || clean.ends_with(" bibliography")
}

/// Check if line is a major non-reference heading that signals the end of references.
fn is_major_non_ref_heading(line: &str) -> bool {
    if !line.starts_with('#') {
        return false;
    }
    let clean = line.trim_start_matches('#').trim().to_lowercase();
    matches!(clean.as_str(), "appendix" | "author contributions" | "acknowledgements" | "acknowledgments")
        || clean.starts_with("appendix")
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

/// Split a raw reference block into individual citation strings.
fn split_raw_entries(block: &str) -> Vec<String> {
    let lines: Vec<&str> = block
        .lines()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !is_noise_line(l))
        .collect();

    if lines.is_empty() {
        return Vec::new();
    }

    let mut entries = Vec::new();
    let mut current = String::new();

    for line in lines {
        if is_new_entry_start(line) {
            if !current.is_empty() {
                entries.push(current.trim().to_string());
                current.clear();
            }
            current.push_str(line);
        } else {
            if !current.is_empty() {
                current.push(' ');
            }
            current.push_str(line);
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

/// Determine if a line starts a new citation entry (e.g. `[1]`, `1.`, `[Vaswani 2017]`).
fn is_new_entry_start(line: &str) -> bool {
    // [1], [12], [Vaswani2017]
    if line.starts_with('[')
        && let Some(close_pos) = line.find(']')
    {
        let inside = &line[1..close_pos];
        if inside.chars().all(|c| c.is_numeric()) || inside.contains("et al") || inside.contains(',') || inside.len() < 30 {
            return true;
        }
    }

    // 1. Author..., 12. Author...
    let first_word = line.split_whitespace().next().unwrap_or("");
    if let Some(num_part) = first_word.strip_suffix('.')
        && !num_part.is_empty()
        && num_part.chars().all(|c| c.is_numeric())
    {
        return true;
    }

    false
}

/// Extract metadata fields (authors, year, title, doi) from a raw citation string.
fn parse_entry_metadata(text: &str) -> (Option<String>, Option<i32>, Option<String>, Option<String>) {
    let doi = extract_doi(text);
    let year = extract_year(text);
    let title = extract_title(text);
    let authors = extract_authors(text, year, title.as_deref());

    (authors, year, title, doi)
}

fn extract_doi(text: &str) -> Option<String> {
    if let Some(pos) = text.find("10.") {
        let candidate = &text[pos..];
        let doi: String = candidate
            .chars()
            .take_while(|c| !c.is_whitespace() && *c != ',' && *c != ';')
            .collect();
        if doi.len() > 7 && doi.contains('/') {
            return Some(doi);
        }
    }
    None
}

fn extract_year(text: &str) -> Option<i32> {
    for word in text.split(&[' ', '(', ')', ',', '.', '[', ']'][..]) {
        if word.len() == 4
            && word.chars().all(|c| c.is_numeric())
            && let Ok(y) = word.parse::<i32>()
            && (1800..=2030).contains(&y)
        {
            return Some(y);
        }
    }
    None
}

fn extract_title(text: &str) -> Option<String> {
    // 1) Look for quoted title: "Title..." or “Title...”
    for (start_quote, end_quote) in [('"', '"'), ('“', '”')] {
        if let Some(start) = text.find(start_quote) {
            let rest = &text[start + start_quote.len_utf8()..];
            if let Some(end) = rest.find(end_quote) {
                let title = rest[..end].trim();
                if title.len() >= 4 {
                    return Some(title.to_string());
                }
            }
        }
    }
    None
}

fn extract_authors(text: &str, year: Option<i32>, title: Option<&str>) -> Option<String> {
    let clean = text.trim();
    if let Some(y) = year {
        let year_str = y.to_string();
        if let Some(pos) = clean.find(&year_str) {
            let candidate = clean[..pos]
                .trim_end_matches(&[' ', '(', ')', ',', '.'][..])
                .trim();
            if !candidate.is_empty() && candidate.len() < 120 {
                return Some(candidate.to_string());
            }
        }
    }

    if let Some(t) = title
        && let Some(pos) = clean.find(t)
    {
        let candidate = clean[..pos].trim_end_matches(&[' ', '"', '“', ',', '.'][..]).trim();
        if !candidate.is_empty() && candidate.len() < 120 {
            return Some(candidate.to_string());
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
        for header in ["# Bibliography", "## Works Cited", "### Literature Cited", "# References and Notes", "## 10. References"] {
            let md = format!(
                "# Intro\nText\n\n{header}\n[1] Author A. \"Title A\" 2021.\n\n# Appendix\nAppendix text"
            );
            let refs = extract_references_block(&md).unwrap();
            assert!(refs.contains("Author A"), "Failed for header: {header}");
            assert!(!refs.contains("Appendix text"), "Failed to terminate on Appendix for header: {header}");
        }
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
