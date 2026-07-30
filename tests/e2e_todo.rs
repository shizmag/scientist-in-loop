use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_sil_todo_e2e() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("my-paper");

    // sil init
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("init").arg("my-paper").current_dir(dir.path()).assert().success();

    // Add # -- X -- # block to paper_draft.tex
    let draft_path = project.join("paper_draft.tex");
    let content = r#"
\section{Methods}
Some draft text.

% # -- X -- #
% TODO: Run benchmark on GPUs.
% # -- X -- #
"#;
    std::fs::write(&draft_path, content).unwrap();

    // sil todo
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("todo").current_dir(&project).assert().success()
        .stdout(predicate::str::contains("Active `# -- X -- #` Idea & TODO Blocks"))
        .stdout(predicate::str::contains("Run benchmark on GPUs."));

    // sil todo --json
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("todo").arg("--json").current_dir(&project).assert().success()
        .stdout(predicate::str::contains("Run benchmark on GPUs."));
}
