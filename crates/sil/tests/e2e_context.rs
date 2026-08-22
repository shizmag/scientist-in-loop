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

#[test]
fn context_json_output() {
    let (_dir, project) = init_project("ctx-json");
    git_commit_all(&project, "Initialize sil project\n\nSci-Action: init\n");

    let output = sil()
        .current_dir(&project)
        .args(["project", "context", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    let val: serde_json::Value = serde_json::from_str(&text).expect("valid json output");
    assert_eq!(val["schema_version"], "sil.dev/agent-state/v1");
    assert_eq!(val["state"], "ready");
    assert!(val.get("project").is_some());
    assert!(val.get("inputs").is_some());
    assert!(val.get("health").is_some());
    assert!(val.get("structure").is_some());
    assert!(val.get("skills").is_some());
    assert!(val.get("capabilities").is_some());
    assert!(val.get("actions").is_some());
}

#[test]
fn context_json_compact_and_envelope() {
    let (_dir, project) = init_project("ctx-json-compact");
    git_commit_all(&project, "Initialize sil project\n\nSci-Action: init\n");

    // Compact JSON (single line)
    let output = sil()
        .current_dir(&project)
        .args(["project", "context", "--json", "--compact"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let text = String::from_utf8(output).unwrap();
    let trimmed = text.trim();
    assert_eq!(trimmed.lines().count(), 1);
    let val: serde_json::Value = serde_json::from_str(trimmed).expect("valid compact json");
    assert_eq!(val["schema_version"], "sil.dev/agent-state/v1");

    // Envelope JSON
    let env_output = sil()
        .current_dir(&project)
        .args(["project", "context", "--json", "--envelope"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let env_text = String::from_utf8(env_output).unwrap();
    let env_val: serde_json::Value = serde_json::from_str(&env_text).expect("valid envelope json");
    assert!(env_val.get("state").is_some());
    assert!(env_val.get("execution").is_some());
    assert_eq!(env_val["state"]["schema_version"], "sil.dev/agent-state/v1");
    assert!(env_val["execution"]["checked_at"].as_str().is_some());
}

#[test]
fn context_json_stable_fingerprint_parity() {
    let (_dir, project) = init_project("ctx-json-parity");
    git_commit_all(&project, "Initialize sil project\n\nSci-Action: init\n");

    let out1 = sil()
        .current_dir(&project)
        .args(["project", "context", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let out2 = sil()
        .current_dir(&project)
        .args(["project", "context", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let str1 = String::from_utf8(out1).unwrap();
    let str2 = String::from_utf8(out2).unwrap();

    let v1: serde_json::Value = serde_json::from_str(&str1).unwrap();
    let v2: serde_json::Value = serde_json::from_str(&str2).unwrap();

    assert_eq!(v1, v2);
}
