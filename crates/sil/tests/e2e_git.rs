//! E2E: commit proposals and `sil log` Sci-Action trailers.

mod common;

use common::{git_commit_all, init_project, sil};

#[test]
fn log_shows_sci_action() {
    let (_dir, project) = init_project("logp");

    git_commit_all(
        &project,
        "Initialize sil project\n\nSci-Action: init\n",
    );

    sil()
        .current_dir(&project)
        .args(["git", "log"])
        .assert()
        .success()
        .stdout(predicates::str::contains("init"))
        .stdout(predicates::str::contains("Initialize"));
}

#[test]
fn init_proposes_sci_action_without_auto_commit() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("nocommit");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("Sci-Action: init"))
        .stdout(predicates::str::contains("not applied"));

    // No commits yet — only git init
    let log = std::process::Command::new("git")
        .args(["-C", project.to_str().unwrap(), "log", "--oneline"])
        .output()
        .unwrap();
    assert!(
        !log.status.success()
            || String::from_utf8_lossy(&log.stdout).trim().is_empty()
            || String::from_utf8_lossy(&log.stderr).contains("does not have any commits")
            || String::from_utf8_lossy(&log.stderr).contains("unknown revision")
            || String::from_utf8_lossy(&log.stderr).contains("bad default revision"),
        "init must not auto-commit; git log should be empty or fail"
    );
}
