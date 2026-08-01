//! Deterministic bibliography / citation suggestion helpers (stub quality).

use serde::{Deserialize, Serialize};

use crate::{SourceDocument, SourceKind};

/// A suggested BibTeX entry and `\cite{...}` form.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BibSuggestion {
    /// Cite key suitable for `\cite{key}`.
    pub cite_key: String,
    /// Full `\cite{key}` command.
    pub cite_command: String,
    /// BibTeX entry body (including `@type{key, ...}`).
    pub bibtex: String,
    /// Human-readable note about how fields were derived.
    pub note: String,
}

/// Build a filesystem/cite-safe key from a title or filename stem.
pub fn slug_cite_key(input: &str) -> String {
    let stem = input
        .trim()
        .trim_end_matches(".pdf")
        .trim_end_matches(".PDF");
    let mut out = String::new();
    let mut prev_us = false;
    for ch in stem.chars() {
        let lower = ch.to_ascii_lowercase();
        if lower.is_ascii_alphanumeric() {
            out.push(lower);
            prev_us = false;
        } else if !prev_us && !out.is_empty() {
            out.push('_');
            prev_us = true;
        }
    }
    let key = out.trim_matches('_').to_string();
    if key.is_empty() {
        "unknown".into()
    } else {
        key.chars()
            .take(64)
            .collect::<String>()
            .trim_end_matches('_')
            .to_string()
    }
}

/// Format a `\cite{key}` command.
pub fn format_cite_command(key: &str) -> String {
    format!("\\cite{{{key}}}")
}

/// Format a minimal `@article` BibTeX entry (deterministic stub).
pub fn format_bibtex_article(
    key: &str,
    title: &str,
    author: &str,
    year: &str,
    journal: &str,
) -> String {
    format!(
        "@article{{{key},\n  title={{{title}}},\n  author={{{author}}},\n  journal={{{journal}}},\n  year={{{year}}}\n}}\n"
    )
}

/// Suggest a citation from a source document.
///
/// Deterministic: same inputs always yield the same key and BibTeX entry.
/// If `authors`, `year`, `venue`, or `doi` are present on `doc`, formats a complete
/// `@article` or `@misc` BibTeX entry with real authors, year, journal/venue, and doi.
pub fn suggest_from_source(doc: &SourceDocument) -> BibSuggestion {
    let display_title = doc
        .title
        .as_deref()
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .unwrap_or_else(|| {
            doc.filename
                .trim_end_matches(".pdf")
                .trim_end_matches(".PDF")
                .trim_end_matches(".md")
                .trim_end_matches(".txt")
                .trim_end_matches(".html")
                .replace(['_', '-'], " ")
        });
    let cite_key = slug_cite_key(doc.title.as_deref().unwrap_or(&doc.filename));
    let author = doc.authors.as_deref().unwrap_or("Unknown");
    let year = doc
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "n.d.".to_string());

    let has_meta =
        doc.authors.is_some() || doc.year.is_some() || doc.venue.is_some() || doc.doi.is_some();

    let (_entry_type, bibtex) = if doc.kind == SourceKind::Dataset || doc.kind == SourceKind::Code {
        let mut fields = vec![
            format!("  title={{{display_title}}}"),
            format!("  author={{{author}}}"),
        ];
        if let Some(venue) = &doc.venue {
            fields.push(format!("  howpublished={{{venue}}}"));
        }
        fields.push(format!("  year={{{year}}}"));
        if let Some(doi) = &doc.doi {
            fields.push(format!("  doi={{{doi}}}"));
        }
        let body = fields.join(",\n");
        ("misc", format!("@misc{{{cite_key},\n{body}\n}}\n"))
    } else {
        let journal = doc.venue.as_deref().unwrap_or("Unknown");
        let mut fields = vec![
            format!("  title={{{display_title}}}"),
            format!("  author={{{author}}}"),
            format!("  journal={{{journal}}}"),
            format!("  year={{{year}}}"),
        ];
        if let Some(doi) = &doc.doi {
            fields.push(format!("  doi={{{doi}}}"));
        }
        let body = fields.join(",\n");
        ("article", format!("@article{{{cite_key},\n{body}\n}}\n"))
    };

    let note = if has_meta {
        format!("BibTeX derived from source metadata ('{}')", doc.filename)
    } else {
        format!(
            "Stub BibTeX from source '{}'; fill author/year/journal before finalizing",
            doc.filename
        )
    };

    BibSuggestion {
        cite_command: format_cite_command(&cite_key),
        cite_key,
        bibtex,
        note,
    }
}

/// Helper to suggest citation from filename and optional title.
pub fn suggest_from_filename_title(filename: &str, title: Option<&str>) -> BibSuggestion {
    let mut doc = SourceDocument::new(filename.into());
    doc.title = title.map(|t| t.to_string());
    suggest_from_source(&doc)
}

/// Suggest a citation from a free-text query (e.g. search hit snippet seed).
pub fn suggest_from_query(query: &str) -> BibSuggestion {
    let key = slug_cite_key(query);
    let title = query.trim();
    let title = if title.is_empty() { "Untitled" } else { title };
    let bibtex = format_bibtex_article(&key, title, "Unknown", "n.d.", "Unknown");
    BibSuggestion {
        cite_command: format_cite_command(&key),
        cite_key: key,
        bibtex,
        note: "Stub BibTeX from query text; refine fields manually".into(),
    }
}

/// Suggest a citation from a parsed ReferenceEntry item.
pub fn suggest_from_reference_entry(entry: &crate::ReferenceEntry) -> BibSuggestion {
    let title = entry.title.as_deref().unwrap_or("Untitled");
    let author = entry.authors.as_deref().unwrap_or("Unknown");
    let year = entry
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "n.d.".into());
    let cite_key = slug_cite_key(if !title.is_empty() && title != "Untitled" {
        title
    } else {
        &entry.raw_text
    });
    let bibtex = format_bibtex_article(&cite_key, title, author, &year, "Extracted Reference");
    BibSuggestion {
        cite_command: format_cite_command(&cite_key),
        cite_key,
        bibtex,
        note: format!(
            "Extracted reference #{} from source '{}'",
            entry.ref_index, entry.source_id
        ),
    }
}

/// Parsed information about a single BibTeX entry block used for matching and deduplication.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct BibEntryInfo {
    /// Cite key in `@type{cite_key,`.
    pub cite_key: Option<String>,
    /// Extracted title field.
    pub title: Option<String>,
    /// Extracted DOI field.
    pub doi: Option<String>,
    /// Extracted arXiv ID / eprint field.
    pub arxiv_id: Option<String>,
    /// True if entry is marked as `unproved, incomplete`.
    pub is_incomplete: bool,
}

/// Extract metadata fields and status from a BibTeX entry block string.
pub fn extract_bib_entry_info(entry_str: &str) -> BibEntryInfo {
    let mut info = BibEntryInfo::default();

    let lower = entry_str.to_lowercase();
    info.is_incomplete = lower.contains("unproved, incomplete")
        || lower.contains("status: unproved")
        || lower.contains("journal={unknown}")
        || lower.contains("author={unknown}");

    for line in entry_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') && trimmed.contains('{') {
            if let Some(start) = trimmed.find('{') {
                let key_part = trimmed[start + 1..].trim();
                let key = key_part.trim_end_matches(',').trim().to_string();
                if !key.is_empty() {
                    info.cite_key = Some(key);
                }
            }
        } else if let Some(eq_pos) = trimmed.find('=') {
            let key = trimmed[..eq_pos].trim().to_lowercase();
            let val = trimmed[eq_pos + 1..]
                .trim()
                .trim_end_matches(',')
                .trim_matches('{')
                .trim_matches('}')
                .trim_matches('"')
                .trim();
            match key.as_str() {
                "title" => info.title = Some(val.to_string()),
                "doi" => info.doi = Some(val.to_string()),
                "eprint" | "arxiv" | "arxiv_id" => info.arxiv_id = Some(val.to_string()),
                _ => {}
            }
        }
    }

    info
}

fn normalize_title(title: &str) -> String {
    title
        .chars()
        .filter(|c| c.is_alphanumeric() || c.is_whitespace())
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Check if two BibTeX entries refer to the same paper.
pub fn is_same_paper(a: &BibEntryInfo, b: &BibEntryInfo) -> bool {
    if let (Some(k1), Some(k2)) = (&a.cite_key, &b.cite_key)
        && k1.to_lowercase() == k2.to_lowercase()
        && k1 != "unknown"
    {
        return true;
    }
    if let (Some(d1), Some(d2)) = (&a.doi, &b.doi)
        && !d1.is_empty()
        && d1.to_lowercase() == d2.to_lowercase()
    {
        return true;
    }
    if let (Some(x1), Some(x2)) = (&a.arxiv_id, &b.arxiv_id) {
        let clean1 = x1
            .trim_start_matches("arxiv:")
            .trim_start_matches("arXiv:")
            .trim();
        let clean2 = x2
            .trim_start_matches("arxiv:")
            .trim_start_matches("arXiv:")
            .trim();
        if !clean1.is_empty() && clean1.to_lowercase() == clean2.to_lowercase() {
            return true;
        }
    }
    if let (Some(t1), Some(t2)) = (&a.title, &b.title) {
        let norm1 = normalize_title(t1);
        let norm2 = normalize_title(t2);
        if !norm1.is_empty() && !norm2.is_empty() {
            if norm1 == norm2 {
                return true;
            }
            if norm1.len() > 15
                && norm2.len() > 15
                && (norm1.contains(&norm2) || norm2.contains(&norm1))
            {
                return true;
            }
        }
    }
    false
}

/// Split a BibTeX file content into individual entry blocks, preserving associated comments.
pub fn parse_bib_blocks(content: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current_block = String::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@')
            && !current_block.trim().is_empty()
            && current_block.contains('@')
        {
            blocks.push(current_block.trim().to_string());
            current_block.clear();
        }
        current_block.push_str(line);
        current_block.push('\n');
    }
    if !current_block.trim().is_empty() {
        blocks.push(current_block.trim().to_string());
    }

    blocks
}

/// Upsert a BibTeX entry into an existing `references.bib` string content.
///
/// If an entry for the same paper already exists:
/// - Replaces the existing entry block with `new_entry` if existing is incomplete or `new_entry` is complete.
/// - Returns `(updated_bib_content, was_replaced)`.
///
/// If no matching entry exists, appends `new_entry` to the end.
pub fn upsert_bib_entry(existing_bib_content: &str, new_entry: &str) -> (String, bool) {
    let new_info = extract_bib_entry_info(new_entry);
    let mut blocks = parse_bib_blocks(existing_bib_content);

    let mut replaced_idx = None;
    for (idx, block) in blocks.iter().enumerate() {
        let existing_info = extract_bib_entry_info(block);
        if is_same_paper(&existing_info, &new_info) {
            replaced_idx = Some(idx);
            break;
        }
    }

    if let Some(idx) = replaced_idx {
        blocks[idx] = new_entry.trim().to_string();
        (blocks.join("\n\n") + "\n", true)
    } else {
        let mut out = existing_bib_content.trim().to_string();
        if !out.is_empty() {
            out.push_str("\n\n");
        }
        out.push_str(new_entry.trim());
        out.push('\n');
        (out, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_filename() {
        assert_eq!(
            slug_cite_key("Attention_Is_All_You_Need.pdf"),
            "attention_is_all_you_need"
        );
        assert_eq!(slug_cite_key("???"), "unknown");
    }

    #[test]
    fn cite_command_format() {
        assert_eq!(format_cite_command("vaswani2017"), "\\cite{vaswani2017}");
    }

    #[test]
    fn suggest_from_source_deterministic() {
        let mut doc = SourceDocument::new("transformer.pdf".into());
        doc.title = Some("Attention Is All You Need".into());
        let a = suggest_from_source(&doc);
        let b = suggest_from_source(&doc);
        assert_eq!(a, b);
        assert!(a.cite_command.starts_with("\\cite{"));
        assert!(a.bibtex.contains("@article{"));
        assert!(a.bibtex.contains("Attention Is All You Need"));
        assert!(!a.cite_key.is_empty());
        assert!(!a.note.is_empty());
    }

    #[test]
    fn suggest_from_source_with_full_metadata() {
        let mut doc = SourceDocument::new("paper.pdf".into());
        doc.title = Some("Deep Residual Learning for Image Recognition".into());
        doc.authors = Some("Kaiming He, Xiangyu Zhang, Shaoqing Ren, Jian Sun".into());
        doc.year = Some(2016);
        doc.venue = Some("CVPR".into());
        doc.doi = Some("10.1109/CVPR.2016.90".into());

        let suggestion = suggest_from_source(&doc);
        assert_eq!(
            suggestion.cite_key,
            "deep_residual_learning_for_image_recognition"
        );
        assert!(
            suggestion
                .bibtex
                .contains("@article{deep_residual_learning_for_image_recognition,")
        );
        assert!(
            suggestion
                .bibtex
                .contains("author={Kaiming He, Xiangyu Zhang, Shaoqing Ren, Jian Sun}")
        );
        assert!(suggestion.bibtex.contains("journal={CVPR}"));
        assert!(suggestion.bibtex.contains("year={2016}"));
        assert!(suggestion.bibtex.contains("doi={10.1109/CVPR.2016.90}"));
    }

    #[test]
    fn test_upsert_bib_entry_replaces_incomplete_entry() {
        let unproved_bib = r#"% [status: unproved, incomplete]
@misc{deepseek_ai_deepseek_r1,
  title={Deepseek-r1: Incentivizing reasoning capability in llms via reinforcement learning},
  author={DeepSeek-AI},
  journal={Unknown},
  year={2025},
  note={unproved, incomplete},
  url={https://arxiv.org/abs/2501.12948}
}
"#;

        let official_bib = r#"@misc{deepseek2025deepseekr1,
  title={Deepseek-r1: Incentivizing reasoning capability in llms via reinforcement learning},
  author={DeepSeek-AI and Daya Guo and Dejian Yang},
  year={2025},
  eprint={2501.12948},
  archivePrefix={arXiv},
  url={https://arxiv.org/abs/2501.12948}
}
"#;

        let (updated, replaced) = upsert_bib_entry(unproved_bib, official_bib);
        assert!(replaced);
        assert!(updated.contains("@misc{deepseek2025deepseekr1,"));
        assert!(updated.contains("author={DeepSeek-AI and Daya Guo and Dejian Yang}"));
        assert!(!updated.contains("note={unproved, incomplete}"));
    }

    #[test]
    fn test_citation_exists_by_doi_case_insensitive() {
        let existing = r#"@article{old_key,
  title={Quantum Supremacy Using a Programmable Superconducting Processor},
  doi={10.1038/s41586-019-1666-5},
  year={2019}
}
"#;
        let new_entry = r#"@article{arute2019quantum,
  title={Quantum supremacy using a programmable superconducting processor},
  author={Arute, Frank and Arya, Kapil and others},
  journal={Nature},
  volume={574},
  doi={10.1038/S41586-019-1666-5},
  year={2019}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(replaced);
        assert!(updated.contains("@article{arute2019quantum,"));
        assert!(!updated.contains("old_key"));
    }

    #[test]
    fn test_citation_exists_by_arxiv_id_formatting() {
        let existing = r#"% [status: unproved, incomplete]
@misc{attention_raw,
  title={Attention is all you need},
  eprint={arXiv:1706.03762v7},
  note={unproved, incomplete}
}
"#;
        let new_entry = r#"@article{vaswani2017attention,
  title={Attention Is All You Need},
  author={Vaswani, Ashish and Shazeer, Noam},
  eprint={1706.03762},
  archivePrefix={arXiv},
  year={2017}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(replaced);
        assert!(updated.contains("@article{vaswani2017attention,"));
        assert!(!updated.contains("attention_raw"));
    }

    #[test]
    fn test_citation_exists_by_title_normalization() {
        let existing = r#"% [status: unproved, incomplete]
@article{he2016deep,
  title={Deep Residual Learning for Image Recognition!},
  author={Unknown},
  note={unproved, incomplete}
}
"#;
        let new_entry = r#"@article{he2016deep_official,
  title={Deep residual learning for image recognition},
  author={He, Kaiming and Zhang, Xiangyu},
  journal={CVPR},
  year={2016}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(replaced);
        assert!(updated.contains("@article{he2016deep_official,"));
        assert!(!updated.contains("note={unproved, incomplete}"));
    }

    #[test]
    fn test_citation_does_not_exist_appends_new() {
        let existing = r#"@article{paper1,
  title={Attention is All You Need},
  year={2017}
}
"#;
        let new_entry = r#"@article{paper2,
  title={BERT: Pre-training of Deep Bidirectional Transformers for Language Understanding},
  author={Devlin, Jacob},
  year={2019}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(!replaced);
        assert!(updated.contains("paper1"));
        assert!(updated.contains("paper2"));
    }

    #[test]
    fn test_upsert_multiple_entries_preserves_unrelated() {
        let existing = r#"@article{paper1,
  title={Paper One},
  year={2020}
}

% [status: unproved, incomplete]
@misc{paper2_raw,
  title={Paper Two: A Survey},
  note={unproved, incomplete}
}

@article{paper3,
  title={Paper Three},
  year={2022}
}
"#;
        let new_entry = r#"@article{paper2_official,
  title={Paper Two: A Survey},
  author={Smith, Alice},
  journal={IEEE},
  year={2021}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(replaced);
        assert!(updated.contains("paper1"));
        assert!(updated.contains("paper2_official"));
        assert!(updated.contains("paper3"));
        assert!(!updated.contains("paper2_raw"));
    }

    #[test]
    fn suggest_from_source_misc_without_venue() {
        let mut doc = SourceDocument::new("dataset.csv".into());
        doc.title = Some("Dataset Title".into());
        doc.authors = Some("Author A".into());
        doc.year = Some(2023);
        let suggestion = suggest_from_source(&doc);
        assert!(suggestion.bibtex.contains("@misc{dataset_title,"));
        assert!(suggestion.bibtex.contains("author={Author A}"));
        assert!(suggestion.bibtex.contains("year={2023}"));
    }

    #[test]
    fn suggest_from_query_nonempty() {
        let s = suggest_from_query("multi-head self-attention");
        assert!(!s.cite_key.is_empty());
        assert!(s.cite_command.contains("\\cite{"));
        assert!(s.bibtex.contains("multi-head self-attention") || s.bibtex.contains("@article"));
    }

    #[test]
    fn bibtex_article_shape() {
        let b = format_bibtex_article("k1", "T", "A", "2020", "J");
        assert!(b.contains("@article{k1,"));
        assert!(b.contains("title={T}"));
        assert!(b.contains("year={2020}"));
    }

    #[test]
    fn reference_entry_to_bibtex() {
        use crate::source::{ReferenceEntry, SourceId};
        let mut entry = ReferenceEntry {
            id: "1".into(),
            source_id: SourceId::new("doc1"),
            ref_index: 1,
            raw_text: "Raw reference text".into(),
            title: Some("A Novel Approach".into()),
            authors: Some("John Doe".into()),
            year: Some(2023),
            venue: Some("Journal of Testing".into()),
            doi: Some("10.1234/test".into()),
            arxiv_id: None,
            url: None,
        };
        let bib = entry.to_bibtex();
        assert!(bib.contains("@article{a_novel_approach,"));
        assert!(bib.contains("title={A Novel Approach}"));
        assert!(bib.contains("author={John Doe}"));
        assert!(bib.contains("journal={Journal of Testing}"));
        assert!(bib.contains("year={2023}"));
        assert!(bib.contains("doi={10.1234/test}"));
        assert!(bib.contains("note={unproved, incomplete}"));
        assert!(bib.contains("% [status: unproved, incomplete]"));

        entry.title = None;
        entry.authors = None;
        entry.year = None;
        entry.venue = None;
        entry.doi = None;
        let bib_fallback = entry.to_bibtex();
        assert!(bib_fallback.contains("@article{raw_reference_text,"));
        assert!(bib_fallback.contains("title={Raw reference text}"));
        assert!(bib_fallback.contains("author={Unknown}"));
        assert!(bib_fallback.contains("journal={Unknown}"));
        assert!(bib_fallback.contains("year={n.d.}"));
        assert!(!bib_fallback.contains("doi="));
        assert!(bib_fallback.contains("note={unproved, incomplete}"));
    }
}
