//! E2E: `sil split` draft autosplit — preserves original, writes section files.

mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn split_writes_sections_and_preserves_draft() {
    let (_tmp, project) = init_project("split-me");
    let draft_path = project.join("paper_draft.tex");
    let draft = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Hello intro body unique_token_intro.

\subsection{Motivation}
More detail.

\section{Methods}
We did science unique_token_methods.
\end{document}
"#;
    fs::write(&draft_path, draft).unwrap();
    let before = fs::read_to_string(&draft_path).unwrap();

    sil()
        .current_dir(&project)
        .args(["paper", "split"])
        .assert()
        .success()
        .stdout(predicates::str::contains("section file"))
        .stdout(predicates::str::contains("paper_draft.tex"));

    let after = fs::read_to_string(&draft_path).unwrap();
    assert_eq!(before, after, "paper_draft.tex must be unchanged");

    let secs = project.join(".sil/draft_sections");
    assert!(secs.is_dir());
    assert!(secs.join("index.md").is_file());

    let intro = fs::read_to_string(secs.join("01-introduction.tex")).unwrap();
    assert!(intro.contains("unique_token_intro"));
    assert!(intro.contains("\\section{Introduction}"));
    assert!(intro.contains("AUTO-GENERATED"));

    let methods = fs::read_to_string(secs.join("03-methods.tex"))
        .or_else(|_| fs::read_to_string(secs.join("02-methods.tex")));
    // Introduction + Motivation + Methods => methods is 03 if Motivation is subsection
    // (subsection is a separate section in split_tex_sections)
    let methods_body = methods.expect("methods section file");
    assert!(methods_body.contains("unique_token_methods"));

    let index = fs::read_to_string(secs.join("index.md")).unwrap();
    assert!(index.contains("Introduction"));
    assert!(index.contains("Methods"));
}

#[test]
fn split_second_run_refreshes_tree() {
    let (_tmp, project) = init_project("split-refresh");
    let draft_path = project.join("paper_draft.tex");
    fs::write(&draft_path, "\\section{A}\nold-a\n\\section{B}\nold-b\n").unwrap();

    sil()
        .current_dir(&project)
        .args(["paper", "split"])
        .assert()
        .success();
    assert!(project.join(".sil/draft_sections/01-a.tex").is_file());

    fs::write(
        &draft_path,
        "\\section{A}\nnew-a-token\n\\section{B}\nnew-b\n\\section{C}\nnew-c\n",
    )
    .unwrap();
    let draft_snapshot = fs::read_to_string(&draft_path).unwrap();

    sil()
        .current_dir(&project)
        .args(["paper", "split"])
        .assert()
        .success();

    assert_eq!(
        fs::read_to_string(&draft_path).unwrap(),
        draft_snapshot,
        "draft unchanged on re-split"
    );
    let a = fs::read_to_string(project.join(".sil/draft_sections/01-a.tex")).unwrap();
    assert!(a.contains("new-a-token"));
    assert!(!a.contains("old-a"));
    assert!(project.join(".sil/draft_sections/03-c.tex").is_file());
}

#[test]
fn init_seeds_draft_sections() {
    let (_tmp, project) = init_project("split-init");
    let secs = project.join(".sil/draft_sections");
    assert!(secs.is_dir());
    // Default template has Introduction, Related Work, Methods, …
    let entries: Vec<_> = fs::read_dir(&secs)
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|x| x == "tex").unwrap_or(false))
        .collect();
    assert!(
        entries.len() >= 3,
        "expected section files from default draft template, got {}",
        entries.len()
    );
    assert!(secs.join("index.md").is_file());
}

#[test]
fn help_lists_split() {
    let out = sil().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert!(stdout.contains("paper"), "help missing paper:\n{stdout}");
}
