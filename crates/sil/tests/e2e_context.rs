//! E2E: `sil context` default sections and flags.

mod common;

use common::{git_commit_all, init_project, sil};

#[test]
fn context_default_and_flags() {
    let (_dir, project) = init_project("ctx");

    git_commit_all(&project, "Initialize sil project\n\nSci-Action: init\n");

    sil()
        .current_dir(&project)
        .args(["project", "context"])
        .assert()
        .success()
        .stdout(predicates::str::contains("SYSTEM RULES FOR THIS PROJECT"))
        .stdout(predicates::str::contains("structure.yaml"))
        .stdout(predicates::str::contains("config.yaml"))
        .stdout(predicates::str::contains("Sources summary"));

    sil()
        .current_dir(&project)
        .args(["project", "context", "--paper", "--agent", "--skill-paper"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Paper content"))
        .stdout(predicates::str::contains("Agent directory"))
        .stdout(predicates::str::contains("Working with the paper"));
}

#[test]
fn context_task_loads_paper_skill() {
    let (_dir, project) = init_project("ctx-task");

    sil()
        .current_dir(&project)
        .args([
            "project",
            "context",
            "--task",
            "edit paper_draft.tex introduction section",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("SYSTEM RULES FOR THIS PROJECT"))
        .stdout(predicates::str::contains("Working with the paper"));
}

#[test]
fn context_skill_agent_code_flag() {
    let (_dir, project) = init_project("ctx-agent");

    sil()
        .current_dir(&project)
        .args(["project", "context", "--skill-agent-code"])
        .assert()
        .success()
        .stdout(predicates::str::contains("agent/README.md"))
        .stdout(predicates::str::contains(
            "Rules for code written by the agent",
        ));
}
