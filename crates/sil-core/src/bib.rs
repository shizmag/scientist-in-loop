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
/// Format a BibTeX entry string cleanly with 2-space indented fields and preserved leading comments.
pub fn pretty_format_bibtex(bibtex: &str) -> String {
    let trimmed = bibtex.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    let lines: Vec<&str> = bibtex.lines().collect();
    let mut leading_comments = Vec::new();
    let mut line_idx = 0;
    while line_idx < lines.len() {
        let line = lines[line_idx];
        let line_trimmed = line.trim();
        if line_trimmed.starts_with('%') || line_trimmed.starts_with('#') {
            leading_comments.push(line_trimmed);
            line_idx += 1;
        } else if line_trimmed.is_empty() {
            line_idx += 1;
        } else if line_trimmed.contains('@') {
            break;
        } else {
            leading_comments.push(line_trimmed);
            line_idx += 1;
        }
    }

    let remaining = if line_idx < lines.len() {
        lines[line_idx..].join("\n")
    } else {
        String::new()
    };

    let at_pos = match remaining.find('@') {
        Some(pos) => pos,
        None => return trimmed.to_string(),
    };

    let entry_str = &remaining[at_pos..];

    let open_pos = match entry_str.find(['{', '(']) {
        Some(pos) => pos,
        None => return trimmed.to_string(),
    };

    let entry_type = entry_str[1..open_pos].trim().to_lowercase();
    if entry_type.is_empty() {
        return trimmed.to_string();
    }

    let body_str = &entry_str[open_pos + 1..];

    let mut key_end_pos = None;
    let mut brace_depth = 0usize;
    let mut in_quotes = false;

    for (i, ch) in body_str.char_indices() {
        match ch {
            '"' if brace_depth == 0 => in_quotes = !in_quotes,
            '{' if !in_quotes => brace_depth += 1,
            '}' if !in_quotes => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                } else {
                    break;
                }
            }
            ',' if brace_depth == 0 && !in_quotes => {
                key_end_pos = Some(i);
                break;
            }
            _ => {}
        }
    }

    let key_end_pos = match key_end_pos {
        Some(pos) => pos,
        None => return trimmed.to_string(),
    };

    let cite_key = body_str[..key_end_pos].trim();
    if cite_key.is_empty() {
        return trimmed.to_string();
    }

    let fields_str = &body_str[key_end_pos + 1..];

    let mut raw_fields = Vec::new();
    let mut current_field = String::new();
    brace_depth = 0;
    in_quotes = false;
    let mut entry_closed = false;

    for ch in fields_str.chars() {
        if entry_closed {
            break;
        }
        match ch {
            '"' if brace_depth == 0 => {
                in_quotes = !in_quotes;
                current_field.push(ch);
            }
            '{' if !in_quotes => {
                brace_depth += 1;
                current_field.push(ch);
            }
            '}' if !in_quotes => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                    current_field.push(ch);
                } else {
                    entry_closed = true;
                }
            }
            ',' if brace_depth == 0 && !in_quotes => {
                let f = current_field.trim().to_string();
                if !f.is_empty() {
                    raw_fields.push(f);
                }
                current_field.clear();
            }
            _ => {
                current_field.push(ch);
            }
        }
    }

    let last_f = current_field.trim().to_string();
    if !last_f.is_empty() {
        raw_fields.push(last_f);
    }

    let mut formatted_fields = Vec::new();
    for raw_field in raw_fields {
        let trimmed_f = raw_field.trim();
        if trimmed_f.is_empty() {
            continue;
        }
        if let Some(eq_pos) = trimmed_f.find('=') {
            let f_key = trimmed_f[..eq_pos].trim().to_lowercase();
            let f_val = trimmed_f[eq_pos + 1..].trim();
            if !f_key.is_empty() && !f_val.is_empty() {
                formatted_fields.push(format!("  {f_key} = {f_val}"));
            }
        }
    }

    let mut out_lines = Vec::new();
    if !leading_comments.is_empty() {
        out_lines.extend(leading_comments.into_iter().map(String::from));
    }

    out_lines.push(format!("@{entry_type}{{{cite_key},"));

    let num_fields = formatted_fields.len();
    for (i, ff) in formatted_fields.into_iter().enumerate() {
        if i + 1 < num_fields {
            out_lines.push(format!("{ff},"));
        } else {
            out_lines.push(ff);
        }
    }

    out_lines.push("}".to_string());
    out_lines.join("\n") + "\n"
}

/// Rewrite the cite key of a BibTeX entry block string to `new_key`.
pub fn rewrite_bib_cite_key(bibtex: &str, new_key: &str) -> String {
    let trimmed_key = new_key.trim();
    if trimmed_key.is_empty() {
        return pretty_format_bibtex(bibtex);
    }

    let at_pos = match bibtex.find('@') {
        Some(pos) => pos,
        None => return pretty_format_bibtex(bibtex),
    };

    let open_pos = match bibtex[at_pos..].find(['{', '(']) {
        Some(pos) => at_pos + pos,
        None => return pretty_format_bibtex(bibtex),
    };

    let body_str = &bibtex[open_pos + 1..];
    let mut key_end_pos = None;
    let mut brace_depth = 0usize;
    let mut in_quotes = false;

    for (i, ch) in body_str.char_indices() {
        match ch {
            '"' if brace_depth == 0 => in_quotes = !in_quotes,
            '{' if !in_quotes => brace_depth += 1,
            '}' if !in_quotes => {
                if brace_depth > 0 {
                    brace_depth -= 1;
                } else {
                    break;
                }
            }
            ',' if brace_depth == 0 && !in_quotes => {
                key_end_pos = Some(i);
                break;
            }
            _ => {}
        }
    }

    let key_end_offset = match key_end_pos {
        Some(pos) => open_pos + 1 + pos,
        None => return pretty_format_bibtex(bibtex),
    };

    let mut result = String::new();
    result.push_str(&bibtex[..open_pos + 1]);
    result.push_str(trimmed_key);
    result.push_str(&bibtex[key_end_offset..]);

    pretty_format_bibtex(&result)
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
        bibtex: pretty_format_bibtex(&bibtex),
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
    let pretty = pretty_format_bibtex(entry_str);
    let target_str = if pretty.trim().is_empty() {
        entry_str
    } else {
        &pretty
    };

    let mut info = BibEntryInfo::default();

    let lower = target_str.to_lowercase();
    info.is_incomplete = lower.contains("unproved, incomplete")
        || lower.contains("status: unproved")
        || lower.contains("journal={unknown}")
        || lower.contains("author={unknown}");

    for line in target_str.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') && trimmed.contains('{') {
            if let Some(start) = trimmed.find('{') {
                let key_part = trimmed[start + 1..].trim();
                let key = key_part
                    .split(',')
                    .next()
                    .unwrap_or(key_part)
                    .trim()
                    .to_string();
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

/// Normalize an arXiv ID by stripping `arxiv:` / `arXiv:` prefixes, URLs, and trailing version tags (`v1`, `v2`).
pub fn normalize_arxiv_id(input: &str) -> String {
    let mut s = input
        .trim()
        .trim_matches('{')
        .trim_matches('}')
        .trim_matches('"')
        .trim();
    if let Some(pos) = s.find("arxiv.org/abs/") {
        s = s[pos + 14..].trim();
    } else if let Some(pos) = s.find("arxiv.org/pdf/") {
        s = s[pos + 14..].trim_end_matches(".pdf").trim();
    }
    if s.get(..6)
        .map(|p| p.eq_ignore_ascii_case("arxiv:"))
        .unwrap_or(false)
    {
        s = s[6..].trim();
    }
    if let Some(v_pos) = s.rfind(['v', 'V'])
        && v_pos > 0
    {
        let suffix = &s[v_pos + 1..];
        if !suffix.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            s = &s[..v_pos];
        }
    }
    s.to_lowercase()
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
        let clean1 = normalize_arxiv_id(x1);
        let clean2 = normalize_arxiv_id(x2);
        if !clean1.is_empty() && clean1 == clean2 {
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
    let mut current_lines: Vec<&str> = Vec::new();
    let mut has_at = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            if has_at {
                let mut split_idx = current_lines.len();
                while split_idx > 0 {
                    let prev_trimmed = current_lines[split_idx - 1].trim();
                    if prev_trimmed.starts_with('%') || prev_trimmed.is_empty() {
                        split_idx -= 1;
                    } else {
                        break;
                    }
                }

                let block1: Vec<&str> = current_lines.drain(..split_idx).collect();
                let block1_str = block1.join("\n");
                if !block1_str.trim().is_empty() {
                    blocks.push(block1_str.trim().to_string());
                }
            }
            has_at = true;
        }
        current_lines.push(line);
    }

    if !current_lines.is_empty() {
        let block_str = current_lines.join("\n");
        if !block_str.trim().is_empty() {
            blocks.push(block_str.trim().to_string());
        }
    }

    blocks
}

/// Upsert a BibTeX entry into an existing `references.bib` string content.
///
/// When an entry for the same paper already exists (`is_same_paper` is true):
/// - Incomplete existing + any new entry -> replaces existing with `new_entry` (`was_replaced = true`).
/// - Complete existing + incomplete new entry -> keeps existing entry to avoid demoting data quality (`was_replaced = false`).
/// - Complete existing + complete new entry -> replaces existing with `new_entry` (`was_replaced = true`).
///
/// When no matching entry exists, appends `new_entry` to the end (`was_replaced = false`).
///
/// Returns `(updated_bib_content, was_replaced)`.
/// Options for upserting BibTeX entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UpsertOptions {
    /// If true and an existing matching entry is replaced, keep the existing entry's cite key
    /// instead of overwriting it with the new entry's cite key.
    pub preserve_cite_key: bool,
}

/// Upsert a BibTeX entry into an existing `references.bib` string content with specified options.
///
/// When an entry for the same paper already exists (`is_same_paper` is true):
/// - Incomplete existing + any new entry -> replaces existing with `new_entry` (`was_replaced = true`).
/// - Complete existing + incomplete new entry -> keeps existing entry to avoid demoting data quality (`was_replaced = false`).
/// - Complete existing + complete new entry -> replaces existing with `new_entry` (`was_replaced = true`).
///
/// When replacing an existing entry and `options.preserve_cite_key` is true:
/// keeps the existing entry's cite key while upgrading fields with official entry and applying pretty-format rules.
///
/// When no matching entry exists, appends `new_entry` to the end (`was_replaced = false`).
///
/// Returns `(updated_bib_content, was_replaced)`.
pub fn upsert_bib_entry_with_options(
    existing_bib_content: &str,
    new_entry: &str,
    options: UpsertOptions,
) -> (String, bool) {
    let pretty_entry = pretty_format_bibtex(new_entry);
    let new_info = extract_bib_entry_info(&pretty_entry);
    let mut blocks = parse_bib_blocks(existing_bib_content);

    for block in &mut blocks {
        let existing_info = extract_bib_entry_info(block);
        if is_same_paper(&existing_info, &new_info) {
            if existing_info.is_incomplete || !new_info.is_incomplete {
                let entry_to_insert = if options.preserve_cite_key {
                    if let Some(ref existing_key) = existing_info.cite_key {
                        rewrite_bib_cite_key(&pretty_entry, existing_key)
                    } else {
                        pretty_entry.clone()
                    }
                } else {
                    pretty_entry.clone()
                };
                *block = entry_to_insert.trim().to_string();
                return (blocks.join("\n\n") + "\n", true);
            } else {
                return (existing_bib_content.to_string(), false);
            }
        }
    }

    let mut out = existing_bib_content.trim().to_string();
    if !out.is_empty() {
        out.push_str("\n\n");
    }
    out.push_str(pretty_entry.trim());
    out.push('\n');
    (out, false)
}

/// Upsert a BibTeX entry into an existing `references.bib` string content.
///
/// Calls `upsert_bib_entry_with_options` with `preserve_cite_key: false`.
pub fn upsert_bib_entry(existing_bib_content: &str, new_entry: &str) -> (String, bool) {
    upsert_bib_entry_with_options(
        existing_bib_content,
        new_entry,
        UpsertOptions {
            preserve_cite_key: false,
        },
    )
}

/// Canonical marker comment for entries added via sil tui.
pub const TUI_ADDED_MARKER: &str = "% [sil: tui-added]";

/// Check if a BibTeX block or string contains the TUI-added marker comment.
pub fn is_tui_added_bib_block(block: &str) -> bool {
    block.to_lowercase().contains("sil: tui-added")
}

/// Prepend `% [sil: tui-added]` comment to a BibTeX entry string if not already present.
/// Idempotent: returns unchanged if the marker is already present.
pub fn mark_tui_added_bib_entry(bibtex: &str) -> String {
    let pretty = pretty_format_bibtex(bibtex);
    let trimmed = pretty.trim();
    if is_tui_added_bib_block(trimmed) {
        return format!("{trimmed}\n");
    }
    format!("{TUI_ADDED_MARKER}\n{trimmed}\n")
}

/// Remove `% [sil: tui-added]` marker line(s) from a BibTeX entry string.
pub fn unmark_tui_added_bib_entry(bibtex: &str) -> String {
    let lines: Vec<&str> = bibtex
        .lines()
        .filter(|line| !line.to_lowercase().contains("sil: tui-added"))
        .collect();
    let result = lines.join("\n");
    let trimmed = result.trim();
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
}

/// Strip all BibTeX entry blocks containing `% [sil: tui-added]` from `bib_content` for release builds.
pub fn strip_tui_added_bib_entries(bib_content: &str) -> String {
    let blocks = parse_bib_blocks(bib_content);
    let retained: Vec<String> = blocks
        .into_iter()
        .filter(|block| !is_tui_added_bib_block(block))
        .map(|b| b.trim().to_string())
        .filter(|b| !b.is_empty())
        .collect();

    if retained.is_empty() {
        String::new()
    } else {
        retained.join("\n\n") + "\n"
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_pretty_format_bibtex_single_line_crossref() {
        let raw = "@article{Vaswani_2017, title={Attention is All you Need}, volume={30}, ISSN={1234}, url={http://example.com}, DOI={10.5555/123}, journal={Advances in NIPS}, author={Vaswani, Ashish and Shazeer, Noam}, year={2017} }";
        let formatted = pretty_format_bibtex(raw);
        let expected = "@article{Vaswani_2017,\n  title = {Attention is All you Need},\n  volume = {30},\n  issn = {1234},\n  url = {http://example.com},\n  doi = {10.5555/123},\n  journal = {Advances in NIPS},\n  author = {Vaswani, Ashish and Shazeer, Noam},\n  year = {2017}\n}\n";
        assert_eq!(formatted, expected);
    }

    #[test]
    fn test_pretty_format_bibtex_preserves_comments() {
        let raw = "% [status: unproved, incomplete]\n# Hash comment\n@misc{key,\n  title={Title},\n  author={Author}\n}";
        let formatted = pretty_format_bibtex(raw);
        assert!(
            formatted.starts_with("% [status: unproved, incomplete]\n# Hash comment\n@misc{key,")
        );
        assert!(formatted.contains("  title = {Title},"));
        assert!(formatted.contains("  author = {Author}"));
        assert!(formatted.ends_with("}\n"));
    }

    #[test]
    fn test_pretty_format_bibtex_unparseable() {
        let raw = "   Just raw text without at symbol   ";
        assert_eq!(pretty_format_bibtex(raw), "Just raw text without at symbol");
    }

    #[test]
    fn test_pretty_format_bibtex_multiline() {
        let raw = "@article{key,\n  AUTHOR={John Doe},\n  TITLE={A Great Paper},\n  YEAR={2024}\n}";
        let formatted = pretty_format_bibtex(raw);
        let expected = "@article{key,\n  author = {John Doe},\n  title = {A Great Paper},\n  year = {2024}\n}\n";
        assert_eq!(formatted, expected);
    }

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
                .contains("author = {Kaiming He, Xiangyu Zhang, Shaoqing Ren, Jian Sun}")
        );
        assert!(suggestion.bibtex.contains("journal = {CVPR}"));
        assert!(suggestion.bibtex.contains("year = {2016}"));
        assert!(suggestion.bibtex.contains("doi = {10.1109/CVPR.2016.90}"));
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
  author = {DeepSeek-AI and Daya Guo and Dejian Yang},
  year={2025},
  eprint={2501.12948},
  archivePrefix={arXiv},
  url={https://arxiv.org/abs/2501.12948}
}
"#;

        let (updated, replaced) = upsert_bib_entry(unproved_bib, official_bib);
        assert!(replaced);
        assert!(updated.contains("@misc{deepseek2025deepseekr1,"));
        assert!(updated.contains("author = {DeepSeek-AI and Daya Guo and Dejian Yang}"));
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
  journal = {CVPR},
  year = {2016}
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
    fn test_rewrite_bib_cite_key() {
        let raw = "@article{Vaswani2017,\n  title={Attention is All you Need},\n  author={Vaswani, Ashish}\n}";
        let rewritten = rewrite_bib_cite_key(raw, "attention_is_all_you_need");
        assert!(rewritten.contains("@article{attention_is_all_you_need,"));
        assert!(rewritten.contains("author = {Vaswani, Ashish}"));
        assert!(!rewritten.contains("Vaswani2017"));
    }

    #[test]
    fn test_rewrite_bib_cite_key_preserves_comments() {
        let raw =
            "% [sil: tui-added]\n@article{Vaswani2017,\n  title={Attention is All you Need}\n}";
        let rewritten = rewrite_bib_cite_key(raw, "attention_is_all_you_need");
        assert!(rewritten.starts_with("% [sil: tui-added]\n@article{attention_is_all_you_need,"));
    }

    #[test]
    fn test_upsert_bib_entry_preserve_cite_key_requirement_4() {
        let stub_bib = r#"% [status: unproved, incomplete]
@article{attention_is_all_you_need,
  title={Attention Is All You Need},
  author={Unknown},
  journal={Unknown},
  year={n.d.}
}
"#;
        let official_bib = r#"@article{Vaswani2017,
  title={Attention Is All You Need},
  author={Vaswani, Ashish and Shazeer, Noam and Parmar, Niki},
  journal={Advances in Neural Information Processing Systems},
  year={2017}
}
"#;

        let (updated, replaced) = upsert_bib_entry_with_options(
            stub_bib,
            official_bib,
            UpsertOptions {
                preserve_cite_key: true,
            },
        );

        assert!(replaced);
        assert!(
            updated.contains("@article{attention_is_all_you_need,"),
            "Expected output to preserve cite key 'attention_is_all_you_need', got:\n{updated}"
        );
        assert!(
            updated.contains("author = {Vaswani, Ashish and Shazeer, Noam and Parmar, Niki}"),
            "Expected official author field, got:\n{updated}"
        );
        assert!(
            updated.contains("journal = {Advances in Neural Information Processing Systems}"),
            "Expected official journal field, got:\n{updated}"
        );
        assert!(updated.contains("year = {2017}"));
        assert!(!updated.contains("Vaswani2017"));
    }

    #[test]
    fn test_upsert_bib_entry_preserve_cite_key_false_replaces_key() {
        let stub_bib = r#"% [status: unproved, incomplete]
@article{attention_is_all_you_need,
  title={Attention Is All You Need},
  author={Unknown}
}
"#;
        let official_bib = r#"@article{Vaswani2017,
  title={Attention Is All You Need},
  author={Vaswani, Ashish},
  year={2017}
}
"#;

        let (updated, replaced) = upsert_bib_entry_with_options(
            stub_bib,
            official_bib,
            UpsertOptions {
                preserve_cite_key: false,
            },
        );

        assert!(replaced);
        assert!(updated.contains("@article{Vaswani2017,"));
        assert!(!updated.contains("attention_is_all_you_need"));
    }

    #[test]
    fn suggest_from_source_misc_without_venue() {
        let mut doc = SourceDocument::new("dataset.csv".into());
        doc.title = Some("Dataset Title".into());
        doc.authors = Some("Author A".into());
        doc.year = Some(2023);
        let suggestion = suggest_from_source(&doc);
        assert!(suggestion.bibtex.contains("@misc{dataset_title,"));
        assert!(suggestion.bibtex.contains("author = {Author A}"));
        assert!(suggestion.bibtex.contains("year = {2023}"));
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

    #[test]
    fn test_mark_tui_added_bib_entry_idempotent() {
        let entry = "@article{key,\n  title={Title}\n}";
        let marked = mark_tui_added_bib_entry(entry);
        assert!(marked.starts_with("% [sil: tui-added]"));
        assert!(is_tui_added_bib_block(&marked));

        let re_marked = mark_tui_added_bib_entry(&marked);
        assert_eq!(marked, re_marked);
        assert_eq!(re_marked.matches("tui-added").count(), 1);
    }

    #[test]
    fn test_unmark_tui_added_bib_entry() {
        let entry = "@article{key,\n  title={Title}\n}";
        let marked = mark_tui_added_bib_entry(entry);
        assert!(is_tui_added_bib_block(&marked));

        let unmarked = unmark_tui_added_bib_entry(&marked);
        assert!(!is_tui_added_bib_block(&unmarked));
        assert!(unmarked.contains("@article{key,"));
        assert!(!unmarked.contains("tui-added"));
    }

    #[test]
    fn test_strip_tui_added_bib_entries_mixed_file() {
        let bib_content = r#"% Preamble comment
@article{normal1,
  title={Normal One},
  author={Author One}
}

% [sil: tui-added]
@article{tui1,
  title={TUI Added One},
  author={Author Two}
}

% [SIL: TUI-ADDED metadata=extra]
@article{tui2,
  title={TUI Added Two},
  author={Author Three}
}

@article{normal2,
  title={Normal Two},
  author={Author Four}
}
"#;

        let stripped = strip_tui_added_bib_entries(bib_content);
        assert!(stripped.contains("normal1"));
        assert!(stripped.contains("normal2"));
        assert!(!stripped.contains("tui1"));
        assert!(!stripped.contains("tui2"));
    }

    #[test]
    fn test_promote_unmark_survives_strip() {
        let entry = "@article{tui_entry,\n  title={TUI Candidate}\n}";
        let marked = mark_tui_added_bib_entry(entry);

        // Before promote: stripped
        let stripped_before = strip_tui_added_bib_entries(&marked);
        assert!(stripped_before.trim().is_empty());

        // Promote: unmark
        let promoted = unmark_tui_added_bib_entry(&marked);

        // After promote: survives strip
        let stripped_after = strip_tui_added_bib_entries(&promoted);
        assert!(stripped_after.contains("tui_entry"));
    }

    #[test]
    fn test_normalize_arxiv_id() {
        assert_eq!(normalize_arxiv_id("arxiv:1234.5678v1"), "1234.5678");
        assert_eq!(normalize_arxiv_id("arXiv:1234.5678"), "1234.5678");
        assert_eq!(normalize_arxiv_id("1234.5678v2"), "1234.5678");
        assert_eq!(normalize_arxiv_id("1234.5678"), "1234.5678");
        assert_eq!(
            normalize_arxiv_id("ARXIV:hep-th/9901001v3"),
            "hep-th/9901001"
        );
        assert_eq!(
            normalize_arxiv_id("https://arxiv.org/abs/2501.12948v1"),
            "2501.12948"
        );
    }

    #[test]
    fn test_is_same_paper_arxiv_normalization() {
        let a = BibEntryInfo {
            arxiv_id: Some("1234.5678v1".into()),
            ..Default::default()
        };
        let b = BibEntryInfo {
            arxiv_id: Some("arXiv:1234.5678".into()),
            ..Default::default()
        };
        assert!(is_same_paper(&a, &b));
    }

    #[test]
    fn test_upsert_matrix_incomplete_existing_incomplete_new() {
        let existing = r#"% [status: unproved, incomplete]
@article{paper1,
  title={Some Paper},
  author={Unknown},
  journal={Unknown}
}
"#;
        let new_entry = r#"% [status: unproved, incomplete]
@article{paper1_new,
  title={Some Paper},
  author={Unknown},
  journal={Draft Venue}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(replaced);
        assert!(updated.contains("paper1_new"));
    }

    #[test]
    fn test_upsert_matrix_incomplete_existing_complete_new() {
        let existing = r#"% [status: unproved, incomplete]
@article{paper1,
  title={Some Paper},
  author={Unknown},
  journal={Unknown}
}
"#;
        let new_entry = r#"@article{paper1_official,
  title={Some Paper},
  author={Alice Smith},
  journal={Top Conference},
  year={2024}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(replaced);
        assert!(updated.contains("paper1_official"));
        assert!(updated.contains("Alice Smith"));
    }

    #[test]
    fn test_upsert_matrix_complete_existing_incomplete_new() {
        let existing = r#"@article{paper1_official,
  title={Some Paper},
  author={Alice Smith},
  journal={Top Conference},
  year={2024}
}
"#;
        let new_entry = r#"% [status: unproved, incomplete]
@article{paper1_stub,
  title={Some Paper},
  author={Unknown},
  journal={Unknown}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(!replaced);
        assert!(updated.contains("paper1_official"));
        assert!(updated.contains("Alice Smith"));
        assert!(!updated.contains("paper1_stub"));
    }

    #[test]
    fn test_upsert_matrix_complete_existing_complete_new() {
        let existing = r#"@article{paper1_v1,
  title={Some Paper},
  author={Alice Smith},
  journal={Preprint},
  year={2024}
}
"#;
        let new_entry = r#"@article{paper1_v2,
  title={Some Paper},
  author={Alice Smith and Bob Jones},
  journal={Journal Version},
  year={2024}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(replaced);
        assert!(updated.contains("paper1_v2"));
        assert!(updated.contains("Bob Jones"));
    }

    #[test]
    fn test_upsert_matrix_no_match_appends() {
        let existing = r#"@article{paper1,
  title={Paper One},
  author={Alice},
  journal={Journal},
  year={2024}
}
"#;
        let new_entry = r#"@article{paper2,
  title={Paper Two},
  author={Bob},
  journal={Journal},
  year={2024}
}
"#;
        let (updated, replaced) = upsert_bib_entry(existing, new_entry);
        assert!(!replaced);
        assert!(updated.contains("paper1"));
        assert!(updated.contains("paper2"));
    }
}
