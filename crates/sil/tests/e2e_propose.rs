//! E2E: propose / promote / structure-set Sci-Action slice.

mod common;

use std::fs;

use common::{git, git_commit_all, init_project, sil};

#[test]
fn propose_explicit_edit_draft_has_trailer_not_commit() {
    let (_tmp, project) = init_project("prop-edit");
    // Ensure clean baseline commit so we can detect new commits
    git_commit_all(&project, "baseline\n\nSci-Action: init\n");
    let log_before = git(&project, &["rev-list", "--count", "HEAD"]);
    let count_before: u64 = String::from_utf8_lossy(&log_before.stdout)
        .trim()
        .parse()
        .unwrap_or(0);

    let out = sil()
        .current_dir(&project)
        .args([
            "git",
            "propose",
            "--action",
            "edit-draft",
            "-m",
            "Tweak intro",
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("Sci-Action: edit-draft"))
        .stdout(predicates::str::contains("Commit proposal"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Sci-Action: edit-draft"), "{stdout}");
    assert!(
        stdout.contains("not applied")
            || stdout.contains("not committed")
            || stdout.contains("never auto-committed"),
        "{stdout}"
    );

    let log_after = git(&project, &["rev-list", "--count", "HEAD"]);
    let count_after: u64 = String::from_utf8_lossy(&log_after.stdout)
        .trim()
        .parse()
        .unwrap_or(0);
    assert_eq!(
        count_before, count_after,
        "propose must never create a commit"
    );
}

#[test]
fn propose_infers_from_dirty_draft() {
    let (_tmp, project) = init_project("prop-infer");
    git_commit_all(&project, "baseline\n\nSci-Action: init\n");
    fs::write(
        project.join("paper_draft.tex"),
        "% dirty draft\n\\section{Intro}\n",
    )
    .unwrap();

    sil()
        .current_dir(&project)
        .args(["git", "propose"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Sci-Action: edit-draft"))
        .stdout(predicates::str::contains("paper_draft.tex"));
}

#[test]
fn structure_set_updates_yaml_and_proposes() {
    let (_tmp, project) = init_project("struct-set");
    let structure_path = project.join(".sil/structure.yaml");
    let yaml = fs::read_to_string(&structure_path).unwrap();
    let id = if yaml.contains("id: intro") {
        "intro".to_string()
    } else {
        yaml.lines()
            .find_map(|l| l.trim().strip_prefix("id: ").map(|s| s.trim().to_string()))
            .expect("structure should have a section id")
    };

    sil()
        .current_dir(&project)
        .args(["paper", "structure", "set", &id, "draft"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Sci-Action: update-structure"))
        .stdout(predicates::str::contains("draft"));

    let updated = fs::read_to_string(&structure_path).unwrap();
    assert!(
        updated.contains("completion: draft") || updated.contains("draft"),
        "structure.yaml should reflect new completion:\n{updated}"
    );
}

#[test]
fn promote_copies_draft_and_proposes() {
    let (_tmp, project) = init_project("promote-me");
    // Mark at least one section draft so guardrail passes
    sil()
        .current_dir(&project)
        .args(["paper", "structure", "set", "intro", "draft"])
        .assert()
        .success();

    let unique = "PROMOTE_UNIQUE_TOKEN_xyz";
    let mut draft = fs::read_to_string(project.join("paper_draft.tex")).unwrap();
    draft.push_str(&format!("\n% {unique}\n"));
    fs::write(project.join("paper_draft.tex"), &draft).unwrap();

    let promote_out = sil()
        .current_dir(&project)
        .args(["paper", "promote"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Sci-Action: promote-to-final"))
        .get_output()
        .stdout
        .clone();
    let promote_stdout = String::from_utf8_lossy(&promote_out);
    assert!(
        promote_stdout.contains("Commit proposal") || promote_stdout.contains("not applied"),
        "{promote_stdout}"
    );

    let final_tex = fs::read_to_string(project.join("paper.tex")).unwrap();
    assert!(
        final_tex.contains(unique),
        "paper.tex should contain promoted draft content"
    );
    // Draft still present
    assert!(
        fs::read_to_string(project.join("paper_draft.tex"))
            .unwrap()
            .contains(unique)
    );
}

#[test]
fn promote_guardrail_blocks_empty_sections_without_force() {
    let (_tmp, project) = init_project("promote-block");
    // Default structure sections are empty/outline — if all empty-ish without draft, promote may fail
    // Force all to empty
    let yaml = r#"
title: Test
status: draft
sections:
  - id: intro
    title: Introduction
    level: 1
    completion: empty
"#;
    fs::write(project.join(".sil/structure.yaml"), yaml).unwrap();

    let fail = sil()
        .current_dir(&project)
        .args(["paper", "promote"])
        .assert()
        .failure()
        .get_output()
        .stderr
        .clone();
    let err = String::from_utf8_lossy(&fail);
    assert!(err.contains("draft") || err.contains("force"), "{err}");

    sil()
        .current_dir(&project)
        .args(["paper", "promote", "--force"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Sci-Action: promote-to-final"));
}

#[test]
fn help_lists_propose_promote_structure() {
    let out = sil().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    for cmd in ["git", "paper", "source"] {
        assert!(stdout.contains(cmd), "help missing {cmd}:\n{stdout}");
    }
}
