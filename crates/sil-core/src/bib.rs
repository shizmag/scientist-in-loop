//! Deterministic bibliography / citation suggestion helpers (stub quality).

use serde::{Deserialize, Serialize};

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
        key.chars().take(64).collect::<String>().trim_end_matches('_').to_string()
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

/// Suggest a citation from a source filename and optional title.
///
/// Deterministic: same inputs always yield the same key and BibTeX stub.
/// Year defaults to empty placeholder when unknown.
pub fn suggest_from_source(filename: &str, title: Option<&str>) -> BibSuggestion {
    let display_title = title
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(|t| t.to_string())
        .unwrap_or_else(|| {
            filename
                .trim_end_matches(".pdf")
                .trim_end_matches(".PDF")
                .replace(['_', '-'], " ")
        });
    let cite_key = slug_cite_key(title.unwrap_or(filename));
    let bibtex = format_bibtex_article(
        &cite_key,
        &display_title,
        "Unknown",
        "n.d.",
        "Unknown",
    );
    BibSuggestion {
        cite_command: format_cite_command(&cite_key),
        cite_key,
        bibtex,
        note: format!(
            "Stub BibTeX from source '{filename}'; fill author/year/journal before finalizing"
        ),
    }
}

/// Suggest a citation from a free-text query (e.g. search hit snippet seed).
pub fn suggest_from_query(query: &str) -> BibSuggestion {
    let key = slug_cite_key(query);
    let title = query.trim();
    let title = if title.is_empty() {
        "Untitled"
    } else {
        title
    };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_from_filename() {
        assert_eq!(slug_cite_key("Attention_Is_All_You_Need.pdf"), "attention_is_all_you_need");
        assert_eq!(slug_cite_key("???"), "unknown");
    }

    #[test]
    fn cite_command_format() {
        assert_eq!(format_cite_command("vaswani2017"), "\\cite{vaswani2017}");
    }

    #[test]
    fn suggest_from_source_deterministic() {
        let a = suggest_from_source("transformer.pdf", Some("Attention Is All You Need"));
        let b = suggest_from_source("transformer.pdf", Some("Attention Is All You Need"));
        assert_eq!(a, b);
        assert!(a.cite_command.starts_with("\\cite{"));
        assert!(a.bibtex.contains("@article{"));
        assert!(a.bibtex.contains("Attention Is All You Need"));
        assert!(!a.cite_key.is_empty());
        assert!(!a.note.is_empty());
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
}
