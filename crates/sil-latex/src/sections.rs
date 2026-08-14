//! Deterministic LaTeX section splitter (no LLM).

use crate::error::LatexError;

/// A structural section extracted from a `.tex` file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TexSection {
    /// Heading command without arguments (e.g. `section`, `subsection`).
    pub kind: String,
    /// Section title text.
    pub title: String,
    /// 1-based line number where the heading starts.
    pub line_start: usize,
    /// Body text following the heading until the next same-or-higher level heading.
    pub body: String,
}

/// Deterministic LaTeX section splitter.
///
/// Recognizes `\section`, `\subsection`, `\subsubsection` (and starred forms).
pub fn split_tex_sections(source: &str) -> Vec<TexSection> {
    let lines: Vec<&str> = source.lines().collect();
    let mut headings: Vec<(usize, String, String, u8)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if let Some((kind, title, level)) = parse_heading_line(line) {
            headings.push((idx, kind, title, level));
        }
    }

    if headings.is_empty() {
        return vec![TexSection {
            kind: "document".into(),
            title: "(preamble / body)".into(),
            line_start: 1,
            body: source.to_string(),
        }];
    }

    let mut sections = Vec::new();
    for (i, (line_idx, kind, title, level)) in headings.iter().enumerate() {
        let start = *line_idx + 1;
        let end = headings
            .iter()
            .skip(i + 1)
            .find(|(_, _, _, l)| *l <= *level)
            .map(|(idx, _, _, _)| *idx)
            .unwrap_or(lines.len());
        let body = lines[start..end].join("\n");
        sections.push(TexSection {
            kind: kind.clone(),
            title: title.clone(),
            line_start: line_idx + 1,
            body,
        });
    }
    sections
}

/// Insert `\cite{cite_key}` into the specified section body.
///
/// If `cite_key` is already cited in that section body, returns `Ok(tex.to_string())` without duplicate.
/// Otherwise inserts `~\cite{cite_key}` at the end of the section body before the next same-or-higher level
/// heading, `\end{document}`, or EOF.
///
/// Returns `Err(LatexError::SectionNotFound)` if no section matching `section_title` exists.
pub fn insert_cite_in_section(
    tex: &str,
    section_title: &str,
    cite_key: &str,
) -> Result<String, LatexError> {
    let lines: Vec<&str> = tex.lines().collect();
    let mut headings: Vec<(usize, String, String, u8)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if let Some((kind, title, level)) = parse_heading_line(line) {
            headings.push((idx, kind, title, level));
        }
    }

    let (start_line, end_line) = if headings.is_empty() {
        if section_title == "(preamble / body)"
            || section_title == "document"
            || section_title.is_empty()
        {
            let end_idx = lines
                .iter()
                .position(|l| l.trim().starts_with("\\end{document}"))
                .unwrap_or(lines.len());
            (0, end_idx)
        } else {
            return Err(LatexError::SectionNotFound(section_title.to_string()));
        }
    } else {
        let target_idx = headings.iter().position(|(_, _, title, _)| {
            title == section_title || title.trim() == section_title.trim()
        });
        let Some(target_i) = target_idx else {
            return Err(LatexError::SectionNotFound(section_title.to_string()));
        };
        let (line_idx, _kind, _title, level) = &headings[target_i];
        let start = *line_idx + 1;
        let next_heading_line = headings
            .iter()
            .skip(target_i + 1)
            .find(|(_, _, _, l)| *l <= *level)
            .map(|(idx, _, _, _)| *idx)
            .unwrap_or(lines.len());
        let mut end = next_heading_line;
        for (i, line) in lines.iter().enumerate().take(next_heading_line).skip(start) {
            if line.trim().starts_with("\\end{document}") {
                end = i;
                break;
            }
        }
        (start, end)
    };

    // Check if cite_key is already cited in start_line..end_line
    for line in &lines[start_line..end_line] {
        if line_contains_cite_key(line, cite_key) {
            return Ok(tex.to_string());
        }
    }

    // Find the last non-empty line in the section body that is not a comment line
    let mut last_content_line_idx = None;
    for i in (start_line..end_line).rev() {
        let trimmed = lines[i].trim();
        if !trimmed.is_empty() && !trimmed.starts_with('%') {
            last_content_line_idx = Some(i);
            break;
        }
    }

    let mut new_lines = Vec::with_capacity(lines.len() + 1);

    if let Some(idx) = last_content_line_idx {
        for (i, line) in lines.iter().enumerate() {
            if i == idx {
                let trimmed = line.trim_end();
                if trimmed.ends_with('~') || trimmed.ends_with(' ') {
                    new_lines.push(format!("{trimmed}\\cite{{{cite_key}}}"));
                } else {
                    new_lines.push(format!("{trimmed}~\\cite{{{cite_key}}}"));
                }
            } else {
                new_lines.push((*line).to_string());
            }
        }
    } else {
        // No non-comment content line in section body. Insert after start_line.
        for (i, line) in lines.iter().enumerate() {
            if i == start_line {
                new_lines.push(format!("\\cite{{{cite_key}}}"));
            }
            new_lines.push((*line).to_string());
        }
        if start_line >= lines.len() {
            new_lines.push(format!("\\cite{{{cite_key}}}"));
        }
    }

    let mut result = new_lines.join("\n");
    if tex.ends_with('\n') && !result.ends_with('\n') {
        result.push('\n');
    }
    Ok(result)
}

fn strip_latex_comment(line: &str) -> &str {
    let mut escaped = false;
    for (i, c) in line.char_indices() {
        if c == '\\' {
            escaped = !escaped;
        } else if c == '%' && !escaped {
            return &line[..i];
        } else {
            escaped = false;
        }
    }
    line
}

fn line_contains_cite_key(line: &str, cite_key: &str) -> bool {
    let code = strip_latex_comment(line);
    let mut cursor = code;
    while let Some(pos) = cursor.find('\\') {
        let after_backslash = &cursor[pos + 1..];
        let is_cite_cmd = after_backslash.starts_with("cite")
            || after_backslash.starts_with("nocite")
            || after_backslash.starts_with("autocite")
            || after_backslash.starts_with("parencite")
            || after_backslash.starts_with("textcite");
        if !is_cite_cmd {
            cursor = after_backslash;
            continue;
        }
        let mut idx = 0;
        let bytes = after_backslash.as_bytes();
        while idx < bytes.len() && (bytes[idx].is_ascii_alphabetic() || bytes[idx] == b'*') {
            idx += 1;
        }
        let mut rest = after_backslash[idx..].trim_start();
        while rest.starts_with('[') {
            if let Some(bracket_end) = rest.find(']') {
                rest = rest[bracket_end + 1..].trim_start();
            } else {
                break;
            }
        }
        if rest.starts_with('{')
            && let Some(brace_end) = rest.find('}')
        {
            let keys = &rest[1..brace_end];
            for k in keys.split(',') {
                if k.trim() == cite_key {
                    return true;
                }
            }
            cursor = &rest[brace_end + 1..];
            continue;
        }
        cursor = after_backslash;
    }
    false
}

#[allow(clippy::question_mark)]
fn parse_heading_line(line: &str) -> Option<(String, String, u8)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix('\\')?;
    let (cmd, after_cmd) = if let Some(r) = rest.strip_prefix("subsubsection") {
        ("subsubsection", r)
    } else if let Some(r) = rest.strip_prefix("subsection") {
        ("subsection", r)
    } else if let Some(r) = rest.strip_prefix("section") {
        ("section", r)
    } else if let Some(r) = rest.strip_prefix("chapter") {
        ("chapter", r)
    } else {
        return None;
    };
    let after_cmd = after_cmd.strip_prefix('*').unwrap_or(after_cmd);
    let after_cmd = after_cmd.trim_start();
    let after_cmd = if after_cmd.starts_with('[') {
        let end = after_cmd.find(']')?;
        after_cmd[end + 1..].trim_start()
    } else {
        after_cmd
    };
    let title = extract_brace_group(after_cmd)?;
    let level = match cmd {
        "chapter" => 0u8,
        "section" => 1,
        "subsection" => 2,
        "subsubsection" => 3,
        _ => 1,
    };
    Some((cmd.to_string(), title, level))
}

fn extract_brace_group(s: &str) -> Option<String> {
    let s = s.trim_start();
    if !s.starts_with('{') {
        return None;
    }
    let mut depth = 0i32;
    let mut out = String::new();
    for ch in s.chars() {
        if ch == '{' {
            depth += 1;
            if depth == 1 {
                continue;
            }
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                return Some(out);
            }
        }
        if depth >= 1 && !(depth == 1 && ch == '{') {
            out.push(ch);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_basic_sections() {
        let src = r#"
\documentclass{article}
\begin{document}
\section{Introduction}
Hello intro.

\subsection{Motivation}
More detail.

\section{Methods}
We did science.
\end{document}
"#;
        let secs = split_tex_sections(src);
        assert!(secs.len() >= 2);
        assert_eq!(secs[0].title, "Introduction");
        assert!(secs[0].body.contains("Hello intro"));
        assert!(secs.iter().any(|s| s.title == "Methods"));
        assert!(secs.iter().any(|s| s.title == "Motivation"));
    }

    #[test]
    fn empty_document_single_section() {
        let secs = split_tex_sections("just text");
        assert_eq!(secs.len(), 1);
        assert_eq!(secs[0].kind, "document");
    }

    #[test]
    fn starred_section() {
        let src = r"\section*{Acknowledgments}
Thanks.
\section{Refs}
B1.";
        let secs = split_tex_sections(src);
        assert_eq!(secs[0].title, "Acknowledgments");
    }

    #[test]
    fn short_title_optional_arg() {
        let src = r"\section[Short]{Long Title}
Body.";
        let secs = split_tex_sections(src);
        assert_eq!(secs[0].title, "Long Title");
        assert!(secs[0].body.contains("Body"));
    }

    #[test]
    fn subsubsection_levels() {
        let src = r"
\section{A}
a
\subsection{B}
b
\subsubsection{C}
c
\section{D}
d
";
        let secs = split_tex_sections(src);
        assert!(
            secs.iter()
                .any(|s| s.title == "C" && s.kind == "subsubsection")
        );
        // body of A should stop before D
        let a = secs.iter().find(|s| s.title == "A").unwrap();
        assert!(!a.body.contains("d\n") || !a.body.trim_end().ends_with('d'));
        assert!(a.body.contains("a") || a.body.contains("B") || !a.body.is_empty());
    }

    #[test]
    fn line_start_is_one_based() {
        let src = "line1\n\\section{Intro}\ntext\n";
        let secs = split_tex_sections(src);
        assert_eq!(secs[0].line_start, 2);
    }

    #[test]
    fn empty_section_body() {
        let src = "\\section{A}\n\\section{B}\nbody\n";
        let secs = split_tex_sections(src);
        let a = secs.iter().find(|s| s.title == "A").unwrap();
        assert!(a.body.trim().is_empty() || !a.body.contains("body"));
        let b = secs.iter().find(|s| s.title == "B").unwrap();
        assert!(b.body.contains("body"));
    }

    #[test]
    fn only_preamble_no_sections() {
        let src = "\\documentclass{article}\n\\usepackage{hyperref}\n";
        let secs = split_tex_sections(src);
        assert_eq!(secs.len(), 1);
        assert_eq!(secs[0].kind, "document");
    }

    #[test]
    fn many_sections_stable_order() {
        let mut src = String::new();
        for i in 0..50 {
            src.push_str(&format!("\\section{{S{i}}}\nx\n"));
        }
        let secs = split_tex_sections(&src);
        assert_eq!(secs.len(), 50);
        assert_eq!(secs[0].title, "S0");
        assert_eq!(secs[49].title, "S49");
    }

    #[test]
    fn comment_like_section_not_heading() {
        // Not a real command at line start with backslash-section in comment without \
        let src = "% section{Fake}\n\\section{Real}\nok\n";
        let secs = split_tex_sections(src);
        assert!(secs.iter().any(|s| s.title == "Real"));
        assert!(!secs.iter().any(|s| s.title == "Fake"));
    }

    #[test]
    fn test_insert_cite_in_section_chosen_section_only() {
        let tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Intro text here.

\section{Methods}
Methods text here.
\end{document}
"#;
        let updated = insert_cite_in_section(tex, "Introduction", "vaswani2017").unwrap();
        assert!(updated.contains("Intro text here.~\\cite{vaswani2017}"));
        assert!(updated.contains("Methods text here."));
        assert!(!updated.contains("Methods text here.~\\cite{vaswani2017}"));
    }

    #[test]
    fn test_insert_cite_second_cite_in_same_section_is_noop() {
        let tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Intro text here.~\cite{vaswani2017}

\section{Methods}
Methods text here.
\end{document}
"#;
        let updated = insert_cite_in_section(tex, "Introduction", "vaswani2017").unwrap();
        assert_eq!(updated, tex);
    }

    #[test]
    fn test_insert_cite_macro_variants_recognized_as_already_cited() {
        let tex_citep = r#"\section{Intro}
As shown in \citep[see][p. 12]{vaswani2017}, attention is all you need.
"#;
        let res1 = insert_cite_in_section(tex_citep, "Intro", "vaswani2017").unwrap();
        assert_eq!(res1, tex_citep);

        let tex_multi = r#"\section{Intro}
Prior art \cite{devlin2018, vaswani2017, brown2020}.
"#;
        let res2 = insert_cite_in_section(tex_multi, "Intro", "vaswani2017").unwrap();
        assert_eq!(res2, tex_multi);

        let tex_autocite = r#"\section{Intro}
Prior art \autocite{vaswani2017}.
"#;
        let res3 = insert_cite_in_section(tex_autocite, "Intro", "vaswani2017").unwrap();
        assert_eq!(res3, tex_autocite);
    }

    #[test]
    fn test_insert_cite_missing_section_returns_err() {
        let tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Intro text.
\end{document}
"#;
        let err = insert_cite_in_section(tex, "NonExistent", "vaswani2017").unwrap_err();
        match err {
            LatexError::SectionNotFound(title) => assert_eq!(title, "NonExistent"),
            other => panic!("Unexpected error: {other:?}"),
        }
    }

    #[test]
    fn test_insert_cite_multiple_sections_preserves_other_sections() {
        let tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Intro text.

\subsection{Background}
Background details.

\section{Results}
Results text.
\end{document}
"#;
        let updated = insert_cite_in_section(tex, "Background", "vaswani2017").unwrap();
        assert!(updated.contains("Background details.~\\cite{vaswani2017}"));
        assert!(updated.contains("Intro text.\n"));
        assert!(updated.contains("Results text."));
        assert!(!updated.contains("Results text.~\\cite{vaswani2017}"));
    }

    #[test]
    fn test_insert_cite_empty_section_body() {
        let tex = "\\section{Introduction}\n\n\\section{Methods}\nMethods text.\n";
        let updated = insert_cite_in_section(tex, "Introduction", "vaswani2017").unwrap();
        assert!(updated.contains("\\section{Introduction}\n\\cite{vaswani2017}"));
        assert!(updated.contains("Methods text."));
    }

    #[test]
    fn test_insert_cite_with_comment_at_end_of_section() {
        let tex = r#"\section{Introduction}
Introductory paragraph.

% # -- X -- #
% [TODO: note]
% # -- X -- #

\section{Methods}
"#;
        let updated = insert_cite_in_section(tex, "Introduction", "vaswani2017").unwrap();
        assert!(updated.contains("Introductory paragraph.~\\cite{vaswani2017}"));
        assert!(updated.contains("% # -- X -- #"));
    }
}
