//! E2E: `sil build` (invokes configured engine or clear error).

mod common;

use common::{init_project, sil};

#[test]
fn build_invokes_engine_or_errors_clearly() {
    let (_dir, project) = init_project("buildp");

    // Default engine is tectonic. On machines with tectonic this succeeds;
    // otherwise we still require a clean, actionable failure.
    let assert = sil().current_dir(&project).arg("build").assert();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    if output.status.success() {
        assert!(
            combined.contains("PDF:") || combined.contains("Built"),
            "success path should mention PDF:\n{combined}"
        );
    } else {
        assert!(
            combined.contains("not found")
                || combined.contains("build failed")
                || combined.contains("LaTeX")
                || combined.contains("tectonic"),
            "failure must be human-readable:\n{combined}"
        );
    }
}

#[test]
fn build_release_strips_idea_blocks_and_creates_submission_zip() {
    use std::fs;

    let (_dir, project) = init_project("buildrel");

    fs::write(
        project.join("paper_draft.tex"),
        r#"\documentclass{article}
\title{Quantum Machine Learning}
\author{Quantum Lab}
\begin{document}
\begin{abstract}
Quantum advantage in learning tasks.
\end{abstract}
\section{Methods}
% # -- X -- #
% TODO: Remove internal draft notes before journal submission
% # -- X -- #
Main method prose goes here.
\bibliography{references}
\end{document}
"#,
    )
    .unwrap();

    fs::write(project.join("references.bib"), "@article{q1, author={A}}").unwrap();
    fs::write(
        project.join("neurips_2024.sty"),
        "\\DeclareOption*{\\OptionNotUsed}\n\\ProcessOptions\\relax",
    )
    .unwrap();

    let cfg_path = project.join(".sil/config.yaml");
    let cfg = fs::read_to_string(&cfg_path).unwrap();
    fs::write(&cfg_path, cfg.replace("template: standard", "template: neurips")).unwrap();

    // Run `sil build release`
    let assert = sil().current_dir(&project).args(["build", "release"]).assert();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    let formatted_tex = project.join("paper_neurips.tex");
    assert!(formatted_tex.is_file(), "paper_neurips.tex must exist:\n{combined}");

    let content = fs::read_to_string(&formatted_tex).unwrap();
    assert!(!content.contains("Remove internal draft notes"), "idea block must be stripped in release");
    assert!(!content.contains("# -- X -- #"), "# -- X -- # block markers must be stripped in release");
    assert!(content.contains("Main method prose goes here."), "prose must be preserved");

    let zip_file = project.join("submission_neurips.zip");
    assert!(zip_file.is_file(), "submission_neurips.zip must be created:\n{combined}");

    // Inspect zip contents
    let f = fs::File::open(&zip_file).unwrap();
    let mut archive = zip::ZipArchive::new(f).unwrap();
    let names: Vec<String> = (0..archive.len())
        .map(|i| archive.by_index(i).unwrap().name().to_string())
        .collect();

    assert!(names.contains(&"paper_neurips.tex".to_string()));
    assert!(names.contains(&"references.bib".to_string()));
    assert!(names.contains(&"neurips_2024.sty".to_string()));
}
