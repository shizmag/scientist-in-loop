//! Manuscript health checking: missing citations, unreferenced labels, undefined refs, word count.

#![allow(clippy::collapsible_if)]

use camino::Utf8Path;

use std::collections::HashSet;
use sil_core::{DiagnosticLevel, HealthDiagnostic, ManuscriptHealthReport};
use crate::error::LatexError;
use crate::idea_parser::parse_idea_blocks;

/// Audit LaTeX manuscript file for missing citations, broken references, and TODO blocks.
pub fn audit_manuscript(
    tex_path: &Utf8Path,
    bib_path: Option<&Utf8Path>,
) -> Result<ManuscriptHealthReport, LatexError> {
    if !tex_path.exists() {
        return Err(LatexError::MainNotFound(tex_path.to_string()));
    }

    let tex_content = std::fs::read_to_string(tex_path).map_err(|e| LatexError::Io {
        path: tex_path.to_string(),
        source: e,
    })?;


    // Parse bib keys if bib_path is present
    let bib_keys = if let Some(bp) = bib_path {
        if bp.exists() {
            let bib_text = std::fs::read_to_string(bp).unwrap_or_default();
            extract_bib_keys(&bib_text)
        } else {
            HashSet::new()
        }
    } else {
        HashSet::new()
    };

    let mut report = ManuscriptHealthReport::default();

    // 1. Audit missing citations
    let cite_keys = extract_cite_keys(&tex_content);
    for (line, key) in cite_keys {
        if !bib_keys.is_empty() && !bib_keys.contains(&key) {
            report.missing_citations_count += 1;
            report.diagnostics.push(HealthDiagnostic {
                level: DiagnosticLevel::Error,
                category: "missing_citation".to_string(),
                line: Some(line),
                message: format!("Citation key '\\cite{{{key}}}' not found in references.bib"),
            });
        }
    }

    // 2. Audit defined labels vs referenced labels
    let defined_labels = extract_defined_labels(&tex_content);
    let referenced_labels = extract_referenced_labels(&tex_content);

    // Unreferenced labels (warnings)
    for (line, label) in &defined_labels {
        if !referenced_labels.iter().any(|(_, r)| r == label) {
            report.unreferenced_labels_count += 1;
            report.diagnostics.push(HealthDiagnostic {
                level: DiagnosticLevel::Warning,
                category: "unreferenced_label".to_string(),
                line: Some(*line),
                message: format!("Label '\\label{{{label}}}' is defined but never referenced (e.g. \\ref{{{label}}})"),
            });
        }
    }

    // Undefined references (errors)
    let defined_set: HashSet<&str> = defined_labels.iter().map(|(_, l)| l.as_str()).collect();
    for (line, ref_key) in &referenced_labels {
        if !defined_set.is_empty() && !defined_set.contains(ref_key.as_str()) {
            report.diagnostics.push(HealthDiagnostic {
                level: DiagnosticLevel::Error,
                category: "undefined_reference".to_string(),
                line: Some(*line),
                message: format!("Reference '\\ref{{{ref_key}}}' targets undefined label '\\label{{{ref_key}}}'"),
            });
        }
    }

    // 3. Count Idea / TODO blocks
    let idea_blocks = parse_idea_blocks(&tex_content);
    report.todo_ideas_count = idea_blocks.len();
    for idea in &idea_blocks {
        report.diagnostics.push(HealthDiagnostic {
            level: DiagnosticLevel::Info,
            category: "idea_block".to_string(),
            line: Some(idea.line_start),
            message: format!("# -- X -- # block: {}", first_line(&idea.content)),
        });
    }

    // 4. Word count calculation (excluding comments & latex macro names)
    report.word_count = count_words(&tex_content);

    Ok(report)
}

fn first_line(s: &str) -> &str {
    s.lines().next().unwrap_or(s).trim()
}

fn extract_bib_keys(bib_text: &str) -> HashSet<String> {
    let mut keys = HashSet::new();
    for line in bib_text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            if let (Some(brace), Some(comma)) = (trimmed.find('{'), trimmed.find(',')) {
                if brace < comma {
                    let key = trimmed[brace + 1..comma].trim().to_string();
                    if !key.is_empty() {
                        keys.insert(key);
                    }
                }
            }
        }
    }
    keys
}

fn extract_cite_keys(tex: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    for (idx, line) in tex.lines().enumerate() {
        let line_num = idx + 1;
        let mut rest = line;
        while let Some(pos) = rest.find("\\cite") {
            let slice = &rest[pos..];
            if let (Some(start), Some(end)) = (slice.find('{'), slice.find('}')) {
                if start < end {
                    let keys_str = &slice[start + 1..end];
                    for k in keys_str.split(',') {
                        let k_clean = k.trim().to_string();
                        if !k_clean.is_empty() {
                            results.push((line_num, k_clean));
                        }
                    }
                    rest = &slice[end + 1..];
                    continue;
                }
            }
            rest = &slice[5..];
        }
    }
    results
}


fn extract_defined_labels(tex: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    for (idx, line) in tex.lines().enumerate() {
        let line_num = idx + 1;
        let mut rest = line;
        while let Some(pos) = rest.find("\\label{") {
            let slice = &rest[pos + 7..];
            if let Some(end) = slice.find('}') {
                let label = slice[..end].trim().to_string();
                if !label.is_empty() {
                    results.push((line_num, label));
                }
                rest = &slice[end + 1..];
            } else {
                break;
            }
        }
    }
    results
}

fn extract_referenced_labels(tex: &str) -> Vec<(usize, String)> {
    let mut results = Vec::new();
    for (idx, line) in tex.lines().enumerate() {
        let line_num = idx + 1;
        for macro_name in ["\\ref{", "\\cref{", "\\autoref{", "\\pageref{"] {
            let mut rest = line;
            while let Some(pos) = rest.find(macro_name) {
                let slice = &rest[pos + macro_name.len()..];
                if let Some(end) = slice.find('}') {
                    let ref_key = slice[..end].trim().to_string();
                    if !ref_key.is_empty() {
                        results.push((line_num, ref_key));
                    }
                    rest = &slice[end + 1..];
                } else {
                    break;
                }
            }
        }
    }
    results
}

fn count_words(tex: &str) -> usize {
    let mut count = 0;
    for line in tex.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('%') || trimmed.starts_with('\\') {
            continue;
        }
        count += trimmed.split_whitespace().filter(|w| !w.starts_with('\\')).count();
    }
    count
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_bib_keys() {
        let bib = r#"
@article{Vaswani2017,
  title={Attention Is All You Need},
  author={Vaswani et al.}
}
@inproceedings{Devlin2019,
  title={BERT}
}
"#;
        let keys = extract_bib_keys(bib);
        assert!(keys.contains("Vaswani2017"));
        assert!(keys.contains("Devlin2019"));
    }

    #[test]
    fn test_extract_cite_and_labels() {
        let tex = r#"
\section{Intro}
As shown in \cite{Vaswani2017, MissingKey}, see Figure~\ref{fig:arch}.
\begin{figure}
\label{fig:arch}
\end{figure}
"#;
        let cites = extract_cite_keys(tex);
        assert_eq!(cites.len(), 2);
        assert_eq!(cites[0].1, "Vaswani2017");
        assert_eq!(cites[1].1, "MissingKey");

        let labels = extract_defined_labels(tex);
        assert_eq!(labels.len(), 1);
        assert_eq!(labels[0].1, "fig:arch");

        let refs = extract_referenced_labels(tex);
        assert_eq!(refs.len(), 1);
        assert_eq!(refs[0].1, "fig:arch");
    }
}
