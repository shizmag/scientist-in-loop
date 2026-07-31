//! E2E: `sil template` and `sil build --release` tests.

mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn template_list_outputs_all_templates() {
    let out = sil().args(["paper", "template", "list"]).assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    for name in ["neurips", "icml", "iclr", "ieee", "arxiv", "standard"] {
        assert!(stdout.contains(name), "missing template {name}:\n{stdout}");
    }
}

#[test]
fn template_apply_generates_formatted_manuscript() {
    let (_tmp, project) = init_project("tmpl-test");

    fs::write(
        project.join("paper_draft.tex"),
        r#"\documentclass{article}
\title{Quantum Generative Modeling}
\author{Quantum Agent Lab}
\begin{document}
\begin{abstract}
Quantum models show promise.
\end{abstract}
\section{Quantum Circuits}
Details of quantum circuits.
\bibliography{references}
\end{document}
"#,
    )
    .unwrap();

    let out = sil()
        .current_dir(&project)
        .args(["paper", "template", "apply", "--target", "neurips"])
        .assert()
        .success();

    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert!(stdout.contains("neurips"), "{stdout}");

    let formatted_path = project.join("paper_neurips.tex");
    assert!(formatted_path.is_file(), "missing paper_neurips.tex");

    let formatted = fs::read_to_string(formatted_path).unwrap();
    assert!(formatted.contains("Quantum Generative Modeling"), "{formatted}");
    assert!(formatted.contains("Quantum Agent Lab"), "{formatted}");
    assert!(formatted.contains("neurips_2024"), "{formatted}");
    assert!(formatted.contains("Quantum Circuits"), "{formatted}");
}

#[test]
fn template_apply_icml_and_arxiv() {
    let (_tmp, project) = init_project("tmpl-icml");

    fs::write(
        project.join("paper_draft.tex"),
        r#"\documentclass{article}
\title{Scalable Optimization}
\author{Optimization Team}
\begin{document}
\begin{abstract}
Fast convergence guarantees.
\end{abstract}
\section{Algorithm}
Gradient descent variant.
\end{document}
"#,
    )
    .unwrap();

    sil()
        .current_dir(&project)
        .args(["paper", "template", "apply", "-t", "icml"])
        .assert()
        .success();
    let icml = fs::read_to_string(project.join("paper_icml.tex")).unwrap();
    assert!(icml.contains("icml2024"), "{icml}");
    assert!(icml.contains("Scalable Optimization"), "{icml}");

    sil()
        .current_dir(&project)
        .args(["paper", "template", "apply", "-t", "arxiv"])
        .assert()
        .success();
    let arxiv = fs::read_to_string(project.join("paper_arxiv.tex")).unwrap();
    assert!(arxiv.contains("hyperref"), "{arxiv}");
    assert!(arxiv.contains("Scalable Optimization"), "{arxiv}");
}
