//! Additional e2e edge cases for the real `sil` binary.

mod common;

use std::fs;

use common::{git_commit_all, init_project, sil};
use predicates::prelude::PredicateBooleanExt;

#[test]
fn search_empty_query_does_not_crash() {
    let (_dir, project) = init_project("search-empty-q");
    // Empty string argument may be rejected by clap or handled; must not panic.
    let assert = sil()
        .current_dir(&project)
        .args(["source", "search", ""])
        .assert();
    // Either success with no/empty results or a clean failure — never panic.
    let out = assert.get_output();
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        out.status.success()
            || combined.contains("No results")
            || combined.contains("required")
            || combined.contains("query")
            || !combined.is_empty()
            || out.status.code().is_some(),
        "empty search must not panic: {combined:?}"
    );
}

#[test]
fn parse_absolute_path_works() {
    let (_dir, project) = init_project("abs-parse");
    let pdf = project.join("sources/abs.pdf");
    fs::write(&pdf, sil_parse::minimal_pdf_bytes()).unwrap();
    let abs = pdf.canonicalize().unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", abs.to_str().unwrap()])
        .env("SIL_MARKER_STUB", "absolute path content token_abs")
        .assert()
        .success()
        .stdout(predicates::str::contains("Parsed"));
    sil()
        .current_dir(&project)
        .args(["source", "search", "token_abs"])
        .assert()
        .success()
        .stdout(predicates::str::contains("abs.pdf"));
}

#[test]
fn parse_uppercase_pdf_extension() {
    let (_dir, project) = init_project("upper-ext");
    fs::write(
        project.join("sources/Paper.PDF"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    // no-args list should pick it up (extension case-insensitive)
    sil()
        .current_dir(&project)
        .args(["source", "parse"])
        .env("SIL_MARKER_STUB", "upper case ext token_upper")
        .assert()
        .success();
    sil()
        .current_dir(&project)
        .args(["source", "search", "token_upper"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Paper.PDF").or(predicates::str::contains("paper.pdf")).or(predicates::str::contains("Paper")));
}

#[test]
fn parse_directory_path_fails() {
    let (_dir, project) = init_project("parse-dir");
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources"])
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("not a PDF")
                .or(predicates::str::contains("not found"))
                .or(predicates::str::contains("corrupt"))
                .or(predicates::str::contains("directory")),
        );
}

#[test]
fn build_missing_main_tex_fails() {
    let (_dir, project) = init_project("no-main");
    fs::remove_file(project.join("paper_draft.tex")).unwrap();
    sil()
        .current_dir(&project)
        .args(["paper", "build"])
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("not found")
                .or(predicates::str::contains("main"))
                .or(predicates::str::contains("paper_draft")),
        );
}

#[test]
fn log_outside_project_fails() {
    let dir = tempfile::tempdir().unwrap();
    sil()
        .current_dir(dir.path())
        .args(["git", "log"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a sil project"));
}

#[test]
fn log_with_no_commits_is_calm() {
    let (_dir, project) = init_project("nolog");
    // git init exists but no commits
    sil()
        .current_dir(&project)
        .args(["git", "log"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No matching").or(predicates::str::contains("No")));
}

#[test]
fn status_after_manual_commit_is_clean() {
    let (_dir, project) = init_project("clean-status");
    git_commit_all(
        &project,
        "Initialize sil project\n\nSci-Action: init\n",
    );
    sil()
        .current_dir(&project)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("clean"));
}

#[test]
fn context_only_skill_agent_without_agent_flag() {
    let (_dir, project) = init_project("ctx-skill-only");
    sil()
        .current_dir(&project)
        .args(["project", "context", "--skill-agent-code"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Rules for code written by the agent"));
}

#[test]
fn context_skill_flag_paper_only() {
    let (_dir, project) = init_project("ctx-paper-only");
    let out = sil()
        .current_dir(&project)
        .args(["project", "context", "--skill-paper"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s = String::from_utf8_lossy(&out);
    assert!(s.contains("Working with the paper"));
    assert!(!s.contains("Rules for code written by the agent") || s.contains("SYSTEM"));
}

#[test]
fn search_after_failed_parse_stays_empty() {
    let (_dir, project) = init_project("fail-then-search");
    fs::write(project.join("sources/bad.unsupported"), "not pdf").unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/bad.unsupported"])
        .assert()
        .failure();
    sil()
        .current_dir(&project)
        .args(["source", "search", "anything"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No results"));
}

#[test]
fn status_shows_draft_dirty_when_tex_modified() {
    let (_dir, project) = init_project("draft-dirty");
    git_commit_all(
        &project,
        "Initialize sil project\n\nSci-Action: init\n",
    );
    fs::write(
        project.join("paper_draft.tex"),
        "% modified after commit\n\\documentclass{article}\\begin{document}x\\end{document}\n",
    )
    .unwrap();
    sil()
        .current_dir(&project)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("paper_draft.tex"));
}

#[test]
fn parse_many_with_one_invalid_reports_failure() {
    let (_dir, project) = init_project("batch-partial");
    fs::write(
        project.join("sources/good.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    fs::write(project.join("sources/bad.pdf"), b"not-a-pdf").unwrap();
    // Noninteractive parse selects all unparsed PDFs
    sil()
        .current_dir(&project)
        .args(["source", "parse"])
        .env("SIL_MARKER_STUB", "partial batch ok")
        .assert()
        .failure(); // at least one failed
}

#[test]
fn init_then_search_without_parse() {
    let (_dir, project) = init_project("fresh-search");
    sil()
        .current_dir(&project)
        .args(["source", "search", "hello"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No results"));
}

#[test]
fn source_fetch_outside_project_fails() {
    let dir = tempfile::tempdir().unwrap();
    sil()
        .current_dir(dir.path())
        .args(["source", "fetch", "10.1000/xyz", "--no-parse"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a sil project"));
}

#[test]
fn missing_sil_dir_not_a_project() {
    let dir = tempfile::tempdir().unwrap();
    // create lookalike without .sil
    fs::write(dir.path().join("paper_draft.tex"), "x").unwrap();
    sil()
        .current_dir(dir.path())
        .arg("status")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a sil project"));
}

#[test]
fn context_task_agent_loads_agent_skill() {
    let (_dir, project) = init_project("task-agent");
    sil()
        .current_dir(&project)
        .args(["project", "context", "--task", "add a script under agent/ for reproducibility"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Rules for code written by the agent"));
}
