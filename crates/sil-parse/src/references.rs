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
    if line.starts_with('[') {
        if let Some(close_pos) = line.find(']') {
            let inside = &line[1..close_pos];
            if inside.chars().all(|c| c.is_numeric()) || inside.contains("et al") || inside.contains(',') || inside.len() < 30 {
                return true;
            }
        }
    }

    // 1. Author..., 12. Author...
    let first_word = line.split_whitespace().next().unwrap_or("");
    if first_word.ends_with('.') {
        let num_part = &first_word[..first_word.len() - 1];
        if !num_part.is_empty() && num_part.chars().all(|c| c.is_numeric()) {
            return true;
        }
    }

    false
}

/// Extract metadata fields (authors, year, title, doi) from a raw citation string.
fn parse_entry_metadata(text: &str) -> (Option<String>, Option<i32>, Option<String>, Option<String>) {
    let doi = extract_doi(text);
    let year = extract_year(text);
    let title = extract_title(text);
    let authors = extract_authors(text, year, &title);

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
        if word.len() == 4 && word.chars().all(|c| c.is_numeric()) {
            if let Ok(y) = word.parse::<i32>() {
                if (1800..=2030).contains(&y) {
                    return Some(y);
                }
            }
        }
    }
    None
}

fn extract_title(text: &str) -> Option<String> {
    // Quoted title: "Title Here" or “Title Here”
    if let Some(start) = text.find('"').or_else(|| text.find('“')) {
        let rest = &text[start + 1..];
        if let Some(end) = rest.find('"').or_else(|| rest.find('”')) {
            let t = rest[..end].trim();
            if !t.is_empty() {
                return Some(t.to_string());
            }
        }
    }
    None
}

fn extract_authors(text: &str, year: Option<i32>, title: &Option<String>) -> Option<String> {
    // Strip leading item number [1] or 1.
    let clean = text
        .trim_start_matches(|c: char| c == '[' || c.is_numeric() || c == ']' || c == '.' || c == ' ')
        .trim();

    if let Some(y) = year {
        let year_str = y.to_string();
        if let Some(pos) = clean.find(&year_str) {
            let candidate = clean[..pos].trim_end_matches(&[' ', '(', ',', '.'][..]).trim();
            if !candidate.is_empty() && candidate.len() < 120 {
                return Some(candidate.to_string());
            }
        }
    }

    if let Some(t) = title {
        if let Some(pos) = clean.find(t) {
            let candidate = clean[..pos].trim_end_matches(&[' ', '"', '“', ',', '.'][..]).trim();
            if !candidate.is_empty() && candidate.len() < 120 {
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
}
