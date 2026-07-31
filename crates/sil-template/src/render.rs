//! Target LaTeX template rendering.

use crate::extractor::ExtractedManuscript;
use crate::template::PaperTemplate;

/// Render an extracted manuscript into the target conference/journal LaTeX template.
pub fn render(template: PaperTemplate, manuscript: &ExtractedManuscript) -> String {
    match template {
        PaperTemplate::Standard => render_standard(manuscript),
        PaperTemplate::Neurips => render_neurips(manuscript),
        PaperTemplate::Icml => render_icml(manuscript),
        PaperTemplate::Iclr => render_iclr(manuscript),
        PaperTemplate::Ieee => render_ieee(manuscript),
        PaperTemplate::Arxiv => render_arxiv(manuscript),
    }
}

fn render_standard(m: &ExtractedManuscript) -> String {
    let bib_file = m.bibliography_file.as_deref().unwrap_or("references");
    let bib_style = m.bibliography_style.as_deref().unwrap_or("plain");
    format!(
        r#"\documentclass{{article}}
\usepackage[utf8]{{inputenc}}
\usepackage{{amsmath,amssymb,amsfonts}}
\usepackage{{graphicx}}
\usepackage{{hyperref}}

\title{{{title}}}
\author{{{author}}}
\date{{\today}}

\begin{{document}}
\maketitle

\begin{{abstract}}
{abstract_text}
\end{{abstract}}

{body}

\bibliographystyle{{{bib_style}}}
\bibliography{{{bib_file}}}

\end{{document}}
"#,
        title = default_if_empty(&m.title, "Working Title"),
        author = default_if_empty(&m.author, "Author Name"),
        abstract_text = m.abstract_text,
        body = m.body_prose,
        bib_style = bib_style,
        bib_file = bib_file
    )
}

fn render_neurips(m: &ExtractedManuscript) -> String {
    let bib_file = m.bibliography_file.as_deref().unwrap_or("references");
    format!(
        r#"\documentclass{{article}}

% NeurIPS 2024 template formatting package
\usepackage[final]{{neurips_2024}}

\usepackage[utf8]{{inputenc}}
\usepackage[T1]{{fontenc}}
\usepackage{{hyperref}}
\usepackage{{url}}
\usepackage{{booktabs}}
\usepackage{{amsfonts}}
\usepackage{{nicefrac}}
\usepackage{{microtype}}
\usepackage{{xcolor}}
\usepackage{{graphicx}}
\usepackage{{amsmath,amssymb}}

\title{{{title}}}

\author{{
  {author}
}}

\begin{{document}}

\maketitle

\begin{{abstract}}
{abstract_text}
\end{{abstract}}

{body}

\bibliographystyle{{plainnat}}
\bibliography{{{bib_file}}}

\end{{document}}
"#,
        title = default_if_empty(&m.title, "Working Title"),
        author = default_if_empty(&m.author, "Author Name \\\\ Institution"),
        abstract_text = m.abstract_text,
        body = m.body_prose,
        bib_file = bib_file
    )
}

fn render_icml(m: &ExtractedManuscript) -> String {
    let bib_file = m.bibliography_file.as_deref().unwrap_or("references");
    format!(
        r#"\documentclass{{article}}

% ICML package
\usepackage{{icml2024}}

\usepackage[utf8]{{inputenc}}
\usepackage{{microtype}}
\usepackage{{graphicx}}
\usepackage{{subfigure}}
\usepackage{{booktabs}}
\usepackage{{hyperref}}
\usepackage{{amsmath,amssymb,amsfonts}}

\icmltitletype{{accepted}}

\begin{{document}}

\twocolumn[
\icmltitle{{{title}}}

\begin{{icmlauthorlist}}
\icmlauthor{{{author}}}{{equal}}
\end{{icmlauthorlist}}

\icmlkeywords{{Machine Learning, Scientist-in-loop, AI}}

\vskip 0.3in
]

\begin{{abstract}}
{abstract_text}
\end{{abstract}}

{body}

\bibliographystyle{{icml2024}}
\bibliography{{{bib_file}}}

\end{{document}}
"#,
        title = default_if_empty(&m.title, "Working Title"),
        author = default_if_empty(&m.author, "Author Name"),
        abstract_text = m.abstract_text,
        body = m.body_prose,
        bib_file = bib_file
    )
}

fn render_iclr(m: &ExtractedManuscript) -> String {
    let bib_file = m.bibliography_file.as_deref().unwrap_or("references");
    format!(
        r#"\documentclass{{article}}

% ICLR conference package
\usepackage{{iclr2024_conference,times}}

\usepackage{{hyperref}}
\usepackage{{url}}
\usepackage{{amsmath,amssymb,amsfonts}}
\usepackage{{graphicx}}
\usepackage{{booktabs}}

\title{{{title}}}

\author{{{author}}}

\newcommand{{\fix}}{{\marginpar{{\FIX}}}}
\newcommand{{\new}}{{\marginpar{{\NEW}}}}

\iclrfinalcopy % Uncomment for camera-ready version

\begin{{document}}

\maketitle

\begin{{abstract}}
{abstract_text}
\end{{abstract}}

{body}

\bibliographystyle{{iclr2024_conference}}
\bibliography{{{bib_file}}}

\end{{document}}
"#,
        title = default_if_empty(&m.title, "Working Title"),
        author = default_if_empty(&m.author, "Author Name"),
        abstract_text = m.abstract_text,
        body = m.body_prose,
        bib_file = bib_file
    )
}

fn render_ieee(m: &ExtractedManuscript) -> String {
    let bib_file = m.bibliography_file.as_deref().unwrap_or("references");
    format!(
        r#"\documentclass[conference]{{IEEEtran}}

\usepackage[utf8]{{inputenc}}
\usepackage{{amsmath,amssymb,amsfonts}}
\usepackage{{algorithmic}}
\usepackage{{graphicx}}
\usepackage{{textcomp}}
\usepackage{{xcolor}}
\usepackage{{cite}}

\begin{{document}}

\title{{{title}}}

\author{{\IEEEauthorblockN{{{author}}}}}

\maketitle

\begin{{abstract}}
{abstract_text}
\end{{abstract}}

{body}

\bibliographystyle{{IEEEtran}}
\bibliography{{{bib_file}}}

\end{{document}}
"#,
        title = default_if_empty(&m.title, "Working Title"),
        author = default_if_empty(&m.author, "Author Name"),
        abstract_text = m.abstract_text,
        body = m.body_prose,
        bib_file = bib_file
    )
}

fn render_arxiv(m: &ExtractedManuscript) -> String {
    let bib_file = m.bibliography_file.as_deref().unwrap_or("references");
    format!(
        r#"\documentclass[11pt,a4paper]{{article}}

\usepackage[margin=1in]{{geometry}}
\usepackage[utf8]{{inputenc}}
\usepackage[T1]{{fontenc}}
\usepackage{{amsmath,amssymb,amsfonts,amsthm}}
\usepackage{{graphicx}}
\usepackage{{booktabs}}
\usepackage{{microtype}}
\usepackage[colorlinks=true,linkcolor=blue,citecolor=blue,urlcolor=blue]{{hyperref}}

\title{{\textbf{{{title}}}}}
\author{{{author}}}
\date{{\today}}

\begin{{document}}

\maketitle

\begin{{abstract}}
\noindent {abstract_text}
\end{{abstract}}

{body}

\bibliographystyle{{plainnat}}
\bibliography{{{bib_file}}}

\end{{document}}
"#,
        title = default_if_empty(&m.title, "Working Title"),
        author = default_if_empty(&m.author, "Author Name"),
        abstract_text = m.abstract_text,
        body = m.body_prose,
        bib_file = bib_file
    )
}

fn default_if_empty<'a>(val: &'a str, fallback: &'a str) -> &'a str {
    if val.trim().is_empty() {
        fallback
    } else {
        val.trim()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn render_all_templates_non_empty() {
        let manuscript = ExtractedManuscript {
            title: "Test Title".into(),
            author: "Author A".into(),
            abstract_text: "Sample abstract.".into(),
            body_prose: "\\section{Intro}\nHello world.".into(),
            bibliography_file: Some("refs".into()),
            bibliography_style: Some("plain".into()),
        };

        for &t in &[
            PaperTemplate::Standard,
            PaperTemplate::Neurips,
            PaperTemplate::Icml,
            PaperTemplate::Iclr,
            PaperTemplate::Ieee,
            PaperTemplate::Arxiv,
        ] {
            let rendered = render(t, &manuscript);
            assert!(
                rendered.contains("Test Title"),
                "template {t:?} missing title"
            );
            assert!(
                rendered.contains("Author A"),
                "template {t:?} missing author"
            );
            assert!(
                rendered.contains("Sample abstract."),
                "template {t:?} missing abstract"
            );
            assert!(
                rendered.contains("Hello world."),
                "template {t:?} missing body"
            );
            assert!(
                rendered.contains("refs"),
                "template {t:?} missing bibliography"
            );
        }
    }
}
