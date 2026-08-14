//! Derived badge calculations for source documents.
//!
//! Computes render-time badges: `parsed` / `unparsed`, `in bib`, and `cited`.
//! No new SQLite schema columns are stored.

use regex::Regex;
use std::collections::HashSet;
use std::sync::LazyLock;

/// Type alias representing a source document record for badge calculation.
pub type SourceRecord = sil_core::SourceDocument;

/// Type alias representing a parsed BibTeX entry for badge calculation.
pub type BibEntry = sil_core::BibEntryInfo;

/// Derived status badges for a source document.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct SourceBadges {
    /// Whether the document content has been parsed and stored in SQLite.
    pub parsed: bool,
    /// Whether the document has a matching BibTeX entry in references.bib.
    pub in_bib: bool,
    /// Whether the document (or its matching bib entry key) is cited in the paper draft LaTeX.
    pub cited: bool,
}

impl SourceBadges {
    /// Derive badges for a source document given project bib entries and draft text.
    pub fn derive(source: &SourceRecord, bib_entries: &[BibEntry], draft_tex: &str) -> Self {
        let parsed = source.parsed;

        let source_bib_sug = sil_core::suggest_from_source(source);
        let source_slug_key = source_bib_sug.cite_key.clone();
        let file_slug_key = sil_core::slug_cite_key(&source.filename);
        let raw_stem_key = source
            .filename
            .trim_end_matches(".pdf")
            .trim_end_matches(".PDF")
            .trim_end_matches(".md")
            .trim_end_matches(".txt")
            .trim_end_matches(".html")
            .to_string();

        let source_doi_norm = source.doi.as_deref().map(normalize_doi);

        let source_bib_info = sil_core::BibEntryInfo {
            cite_key: Some(source_slug_key.clone()),
            title: source.title.clone(),
            doi: source.doi.clone(),
            arxiv_id: if source.filename.to_lowercase().contains("arxiv") {
                sil_regex::extract_arxiv_id(&source.filename)
            } else {
                None
            },
            is_incomplete: false,
        };

        let mut in_bib = false;
        let mut matched_keys = Vec::new();

        for entry in bib_entries {
            let mut is_match = false;

            // 1. DOI match
            if let (Some(s_doi), Some(b_doi)) = (&source_doi_norm, &entry.doi) {
                let b_norm = normalize_doi(b_doi);
                if !s_doi.is_empty() && s_doi == &b_norm {
                    is_match = true;
                }
            }

            // 2. Cite key match
            if !is_match {
                if let Some(b_key) = &entry.cite_key {
                    let b_key_clean = b_key.trim();
                    if !b_key_clean.is_empty()
                        && (b_key_clean.eq_ignore_ascii_case(&source_slug_key)
                            || b_key_clean.eq_ignore_ascii_case(&file_slug_key)
                            || b_key_clean.eq_ignore_ascii_case(&raw_stem_key))
                    {
                        is_match = true;
                    }
                }
            }

            // 3. sil_core::is_same_paper match (title, arxiv, etc.)
            if !is_match && sil_core::is_same_paper(&source_bib_info, entry) {
                is_match = true;
            }

            if is_match {
                in_bib = true;
                if let Some(k) = &entry.cite_key {
                    let k_clean = k.trim();
                    if !k_clean.is_empty() {
                        matched_keys.push(k_clean.to_string());
                    }
                }
            }
        }

        let draft_cite_keys = extract_draft_cite_keys(draft_tex);

        let mut candidate_keys = matched_keys;
        candidate_keys.push(source_slug_key);
        candidate_keys.push(file_slug_key);
        candidate_keys.push(raw_stem_key);

        let cited = candidate_keys.iter().any(|key| {
            draft_cite_keys
                .iter()
                .any(|draft_k| draft_k.eq_ignore_ascii_case(key))
        });

        Self {
            parsed,
            in_bib,
            cited,
        }
    }

    /// Format the derived badges as a compact display string.
    ///
    /// Examples:
    /// - `"[parsed · in bib · cited]"`
    /// - `"[parsed · in bib]"`
    /// - `"[parsed]"`
    /// - `"[unparsed]"`
    pub fn format_badge(&self) -> String {
        if !self.parsed {
            "[unparsed]".to_string()
        } else {
            let mut parts = vec!["parsed"];
            if self.in_bib {
                parts.push("in bib");
            }
            if self.cited {
                parts.push("cited");
            }
            format!("[{}]", parts.join(" · "))
        }
    }
}

impl std::fmt::Display for SourceBadges {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format_badge())
    }
}

/// Standalone helper function to derive badges for a source document.
pub fn derive_source_badges(
    source: &SourceRecord,
    bib_entries: &[BibEntry],
    draft_tex: &str,
) -> SourceBadges {
    SourceBadges::derive(source, bib_entries, draft_tex)
}

/// Extract citation keys from LaTeX citation commands in `tex`.
///
/// Handles `\cite{...}`, `\citep{...}`, `\citet{...}`, `\citeauthor{...}`, `\autocite{...}`,
/// `\parencite{...}`, `\textcite{...}`, `\nocite{...}`, with optional arguments like `[see][p. 12]`.
pub fn extract_draft_cite_keys(tex: &str) -> HashSet<String> {
    let mut keys = HashSet::new();

    static CITE_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r"\\(?:cite[a-zA-Z]*|autocite|parencite|textcite|nocite)\s*(?:\[[^\]]*\]\s*)*\{([^}]+)\}").unwrap()
    });

    for line in tex.lines() {
        let trimmed = line.trim();
        // Skip pure comment lines
        if trimmed.starts_with('%') {
            continue;
        }

        let content_line = if let Some(idx) = find_comment_start(line) {
            &line[..idx]
        } else {
            line
        };

        for cap in CITE_REGEX.captures_iter(content_line) {
            if let Some(m) = cap.get(1) {
                for key in m.as_str().split(',') {
                    let clean = key.trim();
                    if !clean.is_empty() {
                        keys.insert(clean.to_string());
                    }
                }
            }
        }
    }

    keys
}

fn find_comment_start(line: &str) -> Option<usize> {
    let mut prev_char = ' ';
    for (i, c) in line.char_indices() {
        if c == '%' && prev_char != '\\' {
            return Some(i);
        }
        prev_char = c;
    }
    None
}

fn normalize_doi(doi: &str) -> String {
    let mut d = doi.trim();
    if let Some(rest) = d.strip_prefix("https://doi.org/") {
        d = rest;
    } else if let Some(rest) = d.strip_prefix("http://doi.org/") {
        d = rest;
    } else if let Some(rest) = d.strip_prefix("https://dx.doi.org/") {
        d = rest;
    } else if let Some(rest) = d.strip_prefix("http://dx.doi.org/") {
        d = rest;
    } else if let Some(rest) = d.strip_prefix("doi:") {
        d = rest;
    }
    d.trim().to_lowercase()
}
