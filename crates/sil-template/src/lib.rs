//! `sil-template` — LaTeX template collector and formatter for ML/AI paper formats.
//!
//! Extracts structured prose, titles, authors, abstracts, and bibliographies from manuscript `.tex` files
//! (`paper_draft.tex` / `paper.tex`) and renders them into popular conference & journal article templates
//! (NeurIPS, ICML, ICLR, IEEE/CVPR, arXiv, Standard).

#![deny(missing_docs)]

mod extractor;
mod render;
mod template;

pub use extractor::ExtractedManuscript;
pub use render::render;
pub use template::PaperTemplate;

/// Apply a target template to a manuscript `.tex` string and return the rendered document.
pub fn apply_template(template: PaperTemplate, tex_source: &str) -> String {
    let clean_source = sil_latex::strip_idea_blocks(tex_source);
    let manuscript = ExtractedManuscript::parse(&clean_source);
    render(template, &manuscript)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn apply_template_end_to_end() {
        let draft = r#"
\documentclass{article}
\title{Generative Agent Foundations}
\author{Antigravity Team}
\begin{document}
\begin{abstract}
Abstract text goes here.
\end{abstract}
\section{Methods}
Methods section body.
\bibliography{references}
\end{document}
"#;
        let neurips = apply_template(PaperTemplate::Neurips, draft);
        assert!(neurips.contains("Generative Agent Foundations"));
        assert!(neurips.contains("Antigravity Team"));
        assert!(neurips.contains("neurips_2024"));
        assert!(neurips.contains("Methods section body."));
    }

    #[test]
    fn apply_template_strips_idea_blocks() {
        let draft = r#"
\documentclass{article}
\title{Generative Agent Foundations}
\author{Antigravity Team}
\begin{document}
\begin{abstract}
Abstract text goes here.
\end{abstract}
\section{Methods}
% # -- X -- #
% TODO: Remove this internal note before submission
% # -- X -- #
Methods section body.
\bibliography{references}
\end{document}
"#;
        let neurips = apply_template(PaperTemplate::Neurips, draft);
        assert!(!neurips.contains("TODO: Remove this internal note"));
        assert!(!neurips.contains("# -- X -- #"));
        assert!(neurips.contains("Methods section body."));
    }
}
