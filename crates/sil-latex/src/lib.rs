//! LaTeX engine abstraction and deterministic section splitting.
//!
//! Stage 0: section splitter + engine command builder.
//! Stage 5: full `sil build` integration.

#![deny(missing_docs)]

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{LatexEngine, SilError};
use thiserror::Error;

/// LaTeX build errors.
#[derive(Debug, Error)]
pub enum LatexError {
    /// Engine binary not found.
    #[error(
        "LaTeX engine '{engine}' not found on PATH; install it or change latex.engine in config"
    )]
    EngineNotFound {
        /// Engine name.
        engine: String,
    },
    /// Compilation failed.
    #[error("LaTeX build failed ({engine}): {message}")]
    BuildFailed {
        /// Engine name.
        engine: String,
        /// Error detail.
        message: String,
    },
    /// Main file missing.
    #[error("main LaTeX file not found: {0}")]
    MainNotFound(String),
    /// Other.
    #[error("{0}")]
    Message(String),
}

impl From<LatexError> for SilError {
    fn from(value: LatexError) -> Self {
        SilError::Build(value.to_string())
    }
}

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

/// Deterministic LaTeX section splitter (no LLM).
///
/// Recognizes `\section`, `\subsection`, `\subsubsection` (and starred forms).
pub fn split_tex_sections(source: &str) -> Vec<TexSection> {
    let heading_re = regex_lite_heading();
    let lines: Vec<&str> = source.lines().collect();
    let mut headings: Vec<(usize, String, String, u8)> = Vec::new();

    for (idx, line) in lines.iter().enumerate() {
        if let Some((kind, title, level)) = parse_heading_line(line, heading_re) {
            headings.push((idx, kind, title, level));
        }
    }

    if headings.is_empty() {
        // Whole document as one implicit section.
        return vec![TexSection {
            kind: "document".into(),
            title: "(preamble / body)".into(),
            line_start: 1,
            body: source.to_string(),
        }];
    }

    let mut sections = Vec::new();
    for (i, (line_idx, kind, title, level)) in headings.iter().enumerate() {
        let start = *line_idx + 1; // body starts after heading line
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

fn regex_lite_heading() -> &'static str {
    // pattern documented for parse_heading_line
    r"section"
}

fn parse_heading_line(line: &str, _re: &str) -> Option<(String, String, u8)> {
    let trimmed = line.trim();
    // Match \section{...}, \section*{...}, \subsection, \subsubsection
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
    // Optional short title [..] then {title}
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
    for (i, ch) in s.chars().enumerate() {
        if ch == '{' {
            depth += 1;
            if depth == 1 {
                continue;
            }
        } else if ch == '}' {
            depth -= 1;
            if depth == 0 {
                let _ = i;
                return Some(out);
            }
        }
        if depth >= 1 {
            // include nested braces content
            if !(depth == 1 && ch == '{') {
                out.push(ch);
            }
        }
    }
    None
}

/// Build a `Command` for the configured engine (does not execute).
pub fn build_command(
    engine: LatexEngine,
    main: &Utf8Path,
    workdir: &Utf8Path,
) -> Result<Command, LatexError> {
    if !main.exists() && !workdir.join(main.as_str()).exists() {
        // allow relative main against workdir
        let candidate = if main.is_absolute() {
            main.to_path_buf()
        } else {
            workdir.join(main)
        };
        if !candidate.exists() {
            return Err(LatexError::MainNotFound(main.to_string()));
        }
    }
    let mut cmd = Command::new(engine.command());
    cmd.current_dir(workdir.as_str());
    match engine {
        LatexEngine::Tectonic => {
            cmd.arg(main.file_name().unwrap_or(main.as_str()));
        }
        LatexEngine::Latexmk => {
            cmd.args(["-pdf", "-interaction=nonstopmode"]);
            cmd.arg(main.file_name().unwrap_or(main.as_str()));
        }
        LatexEngine::Pdflatex | LatexEngine::Xelatex | LatexEngine::Lualatex => {
            cmd.arg("-interaction=nonstopmode");
            cmd.arg(main.file_name().unwrap_or(main.as_str()));
        }
    }
    Ok(cmd)
}

/// Compile the main document with the given engine.
pub fn build(
    engine: LatexEngine,
    main: &Utf8Path,
    workdir: &Utf8Path,
) -> Result<Utf8PathBuf, LatexError> {
    // Check engine presence.
    let which = Command::new(engine.command())
        .arg("--version")
        .output();
    if which.is_err() {
        // try -v
        let which2 = Command::new(engine.command()).arg("-v").output();
        if which2.is_err() {
            return Err(LatexError::EngineNotFound {
                engine: engine.to_string(),
            });
        }
    }

    let main_path = if main.is_absolute() {
        main.to_path_buf()
    } else {
        workdir.join(main)
    };
    if !main_path.exists() {
        return Err(LatexError::MainNotFound(main_path.to_string()));
    }

    let mut cmd = build_command(engine, &main_path, workdir)?;
    let output = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            LatexError::EngineNotFound {
                engine: engine.to_string(),
            }
        } else {
            LatexError::Message(e.to_string())
        }
    })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let stdout = String::from_utf8_lossy(&output.stdout);
        return Err(LatexError::BuildFailed {
            engine: engine.to_string(),
            message: format!("{}\n{}", stderr.trim(), stdout.trim()),
        });
    }
    // Best-effort PDF path.
    let pdf = main_path.with_extension("pdf");
    Ok(pdf)
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
        // subsection is separate
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
}
