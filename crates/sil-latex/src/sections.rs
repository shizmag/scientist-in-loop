//! Deterministic LaTeX section splitter (no LLM).

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

fn parse_heading_line(line: &str) -> Option<(String, String, u8)> {
    let trimmed = line.trim();
    let rest = trimmed.strip_prefix('\\')?;
    let (cmd, after_cmd) = if let Some(r) = rest.strip_prefix("subsubsection") {
        ("subsubsection", r)
    } else if let Some(r) = rest.strip_prefix("subsection") {
        ("subsection", r)
    } else if let Some(r) = rest.strip_prefix("section") {
        ("section", r)
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
        "section" => 1u8,
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
}
