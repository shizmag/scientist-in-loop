//! Extract structured manuscript content from a LaTeX document.

/// Structured manuscript components extracted from a `.tex` file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ExtractedManuscript {
    /// Paper title.
    pub title: String,
    /// Paper author(s).
    pub author: String,
    /// Abstract text body.
    pub abstract_text: String,
    /// Main manuscript prose (sections, math, figures, tables).
    pub body_prose: String,
    /// Bibliography database file name without extension.
    pub bibliography_file: Option<String>,
    /// Bibliography style name.
    pub bibliography_style: Option<String>,
}

impl ExtractedManuscript {
    /// Parse raw `.tex` source into structured components.
    pub fn parse(tex_content: &str) -> Self {
        let title = extract_macro_arg(tex_content, "title").unwrap_or_default();
        let author = extract_macro_arg(tex_content, "author").unwrap_or_default();
        let abstract_text = extract_environment(tex_content, "abstract").unwrap_or_default();
        let bib_file = extract_macro_arg(tex_content, "bibliography");
        let bib_style = extract_macro_arg(tex_content, "bibliographystyle");

        let body_prose = extract_body_prose(tex_content);

        Self {
            title,
            author,
            abstract_text,
            body_prose,
            bibliography_file: bib_file,
            bibliography_style: bib_style,
        }
    }
}

/// Extract single argument from macro `\name{arg}` handling multiline / curly braces.
fn extract_macro_arg(content: &str, macro_name: &str) -> Option<String> {
    let pattern = format!("\\{macro_name}");
    let mut idx = 0;
    while let Some(pos) = content[idx..].find(&pattern) {
        let start = idx + pos + pattern.len();
        let rest = &content[start..];
        let trimmed_rest = rest.trim_start();
        if trimmed_rest.starts_with('{') {
            let brace_start = rest.find('{')? + start;
            if let Some(end) = find_matching_brace(content, brace_start) {
                let inside = &content[brace_start + 1..end];
                return Some(inside.trim().to_string());
            }
        }
        idx = start;
    }
    None
}

/// Extract content inside `\begin{name} ... \end{name}`.
fn extract_environment(content: &str, env_name: &str) -> Option<String> {
    let begin_tag = format!("\\begin{{{env_name}}}");
    let end_tag = format!("\\end{{{env_name}}}");

    let start_pos = content.find(&begin_tag)?;
    let body_start = start_pos + begin_tag.len();
    let end_pos = content[body_start..].find(&end_tag)?;
    let inside = &content[body_start..body_start + end_pos];
    Some(inside.trim().to_string())
}

/// Extract main prose between `\begin{document}` and `\end{document}`,
/// stripping out top-level `\maketitle`, `\begin{abstract}`, `\bibliography`, etc.
fn extract_body_prose(content: &str) -> String {
    let doc_body = extract_environment(content, "document").unwrap_or_else(|| content.to_string());

    let mut lines = Vec::new();
    let mut in_abstract = false;

    for line in doc_body.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("\\begin{abstract}") {
            in_abstract = true;
            continue;
        }
        if in_abstract {
            if trimmed.starts_with("\\end{abstract}") {
                in_abstract = false;
            }
            continue;
        }

        if trimmed == "\\maketitle"
            || trimmed.starts_with("\\title{")
            || trimmed.starts_with("\\author{")
            || trimmed.starts_with("\\date{")
            || trimmed.starts_with("\\bibliography{")
            || trimmed.starts_with("\\bibliographystyle{")
        {
            continue;
        }

        lines.push(line);
    }

    lines.join("\n").trim().to_string()
}

/// Find index of closing matching brace for `{` at `start_idx`.
fn find_matching_brace(s: &str, start_idx: usize) -> Option<usize> {
    let bytes = s.as_bytes();
    if start_idx >= bytes.len() || bytes[start_idx] != b'{' {
        return None;
    }
    let mut depth = 0;
    for (i, &b) in bytes[start_idx..].iter().enumerate() {
        if b == b'{' {
            depth += 1;
        } else if b == b'}' {
            depth -= 1;
            if depth == 0 {
                return Some(start_idx + i);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_sample_manuscript() {
        let tex = r#"
\documentclass{article}
\title{Deep Learning Diagnostics}
\author{Alice & Bob}
\begin{document}
\maketitle

\begin{abstract}
We present a new algorithm.
\end{abstract}

\section{Introduction}
First section prose.

\bibliographystyle{plain}
\bibliography{references}
\end{document}
"#;
        let ext = ExtractedManuscript::parse(tex);
        assert_eq!(ext.title, "Deep Learning Diagnostics");
        assert_eq!(ext.author, "Alice & Bob");
        assert_eq!(ext.abstract_text, "We present a new algorithm.");
        assert!(ext.body_prose.contains("\\section{Introduction}"));
        assert!(ext.body_prose.contains("First section prose."));
        assert!(!ext.body_prose.contains("\\maketitle"));
        assert!(!ext.body_prose.contains("We present a new algorithm."));
        assert_eq!(ext.bibliography_file.as_deref(), Some("references"));
        assert_eq!(ext.bibliography_style.as_deref(), Some("plain"));
    }
}
