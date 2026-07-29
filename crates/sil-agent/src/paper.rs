//! Paper subsection extraction helpers for agent context.

use sil_latex::{TexSection, split_tex_sections};

/// Split paper_draft.tex source into deterministic subsections.
pub fn paper_subsections(tex: &str) -> Vec<TexSection> {
    split_tex_sections(tex)
}

/// Format subsections as markdown for context dumps.
pub fn format_subsections_markdown(sections: &[TexSection]) -> String {
    let mut out = String::new();
    for sec in sections {
        out.push_str(&format!(
            "### \\{}{{{}}}  (line {})\n\n",
            sec.kind, sec.title, sec.line_start
        ));
        let body = sec.body.trim();
        if body.is_empty() {
            out.push_str("_empty_\n\n");
        } else {
            out.push_str("```tex\n");
            out.push_str(body);
            out.push_str("\n```\n\n");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paper_subsections_delegates() {
        let secs = paper_subsections("\\section{A}\nbody\n");
        assert_eq!(secs.len(), 1);
        assert_eq!(secs[0].title, "A");
    }

    #[test]
    fn format_empty_and_body() {
        let secs = vec![
            TexSection {
                kind: "section".into(),
                title: "Empty".into(),
                line_start: 1,
                body: "  ".into(),
            },
            TexSection {
                kind: "section".into(),
                title: "Full".into(),
                line_start: 3,
                body: "text here".into(),
            },
        ];
        let md = format_subsections_markdown(&secs);
        assert!(md.contains("_empty_"));
        assert!(md.contains("```tex"));
        assert!(md.contains("text here"));
        assert!(md.contains("Empty"));
    }
}

