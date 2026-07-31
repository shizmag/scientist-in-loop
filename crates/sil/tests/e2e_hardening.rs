//! High-value e2e hardening: errors, idempotency, context edges, search edges.

mod common;

use std::fs;

use common::{init_project, sil};
use predicates::prelude::PredicateBooleanExt;

// ── A. Error / negative paths ─────────────────────────────────────────────

#[test]
fn commands_outside_project_fail_clearly() {
    let dir = tempfile::tempdir().unwrap();
    for args in [
        vec!["source", "parse"],
        vec!["source", "search", "q"],
        vec!["git", "log"],
        vec!["project", "context"],
        vec!["paper", "build"],
    ] {
        sil()
            .current_dir(dir.path())
            .args(&args)
            .assert()
            .failure()
            .stderr(predicates::str::contains("not a sil project"));
    }
}

#[test]
fn invalid_config_yaml_fails_status() {
    let (_dir, project) = init_project("bad-cfg");
    fs::write(project.join(".sil/config.yaml"), "project: [\n  broken").unwrap();
    sil()
        .current_dir(&project)
        .arg("status")
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("invalid")
                .or(predicates::str::contains("config"))
                .or(predicates::str::contains("YAML"))
                .or(predicates::str::contains("yaml")),
        );
}

#[test]
fn invalid_structure_completion_fails_status() {
    let (_dir, project) = init_project("bad-struct");
    let mut yaml = fs::read_to_string(project.join(".sil/structure.yaml")).unwrap();
    // First completion in template is "empty"
    yaml = yaml.replacen("completion: empty", "completion: done", 1);
    fs::write(project.join(".sil/structure.yaml"), yaml).unwrap();
    sil().current_dir(&project).arg("status").assert().failure();
}

#[test]
fn marker_stub_failure_via_failing_env_script() {
    // When SIL_MARKER_STUB is unset and script fails — use a fake script that exits 1.
    let (_dir, project) = init_project("marker-fail");
    fs::write(
        project.join("sources/a.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();

    let script = project.join("fail_marker.py");
    fs::write(
        &script,
        "#!/usr/bin/env python3\nimport sys\nprint('garbage', file=sys.stderr)\nsys.exit(1)\n",
    )
    .unwrap();

    sil()
        .current_dir(&project)
        .env_remove("SIL_MARKER_STUB")
        .env("SIL_PARSE_SCRIPT", script.to_str().unwrap())
        .args(["source", "parse", "sources/a.pdf"])
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("Marker")
                .or(predicates::str::contains("marker"))
                .or(predicates::str::contains("parse")),
        );
}

// ── B. parse no-args edges ────────────────────────────────────────────────

#[test]
fn parse_no_args_when_all_already_parsed() {
    let (_dir, project) = init_project("all-parsed");
    fs::write(
        project.join("sources/only.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/only.pdf"])
        .env("SIL_MARKER_STUB", "once")
        .assert()
        .success();

    sil()
        .current_dir(&project)
        .args(["source", "parse"])
        .env("SIL_MARKER_STUB", "again")
        .assert()
        .success()
        .stdout(
            predicates::str::contains("Nothing to parse")
                .or(predicates::str::contains("No unparsed")),
        );
}

#[test]
fn parse_no_args_only_unparsed_in_mix() {
    let (_dir, project) = init_project("mix-parse");
    for name in ["done.pdf", "todo.pdf"] {
        fs::write(
            project.join("sources").join(name),
            sil_parse::minimal_pdf_bytes(),
        )
        .unwrap();
    }
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/done.pdf"])
        .env("SIL_MARKER_STUB", "done content")
        .assert()
        .success();

    sil()
        .current_dir(&project)
        .args(["source", "parse"])
        .env("SIL_MARKER_STUB", "todo unique_mix_token_99")
        .assert()
        .success()
        .stdout(predicates::str::contains("parsed"));

    // Only todo content should be searchable as the new token
    sil()
        .current_dir(&project)
        .args(["source", "search", "unique_mix_token_99"])
        .assert()
        .success()
        .stdout(predicates::str::contains("todo.pdf"));
}

// ── C. context ────────────────────────────────────────────────────────────

#[test]
fn context_missing_paper_draft() {
    let (_dir, project) = init_project("no-draft");
    fs::remove_file(project.join("paper_draft.tex")).unwrap();
    sil()
        .current_dir(&project)
        .args(["project", "context", "--paper"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Paper content"))
        .stdout(
            predicates::str::contains("Could not read")
                .or(predicates::str::contains("paper_draft")),
        );
}

#[test]
fn context_large_draft_ok() {
    let (_dir, project) = init_project("big-draft");
    let mut tex = String::from("\\documentclass{article}\n\\begin{document}\n");
    for i in 0..100 {
        tex.push_str(&format!("\\section{{S{i}}}\n"));
        tex.push_str(&"lorem ".repeat(40));
        tex.push('\n');
    }
    tex.push_str("\\end{document}\n");
    fs::write(project.join("paper_draft.tex"), tex).unwrap();

    sil()
        .current_dir(&project)
        .args(["project", "context", "--paper"])
        .assert()
        .success()
        .stdout(predicates::str::contains("S0"))
        .stdout(predicates::str::contains("S99"));
}

// ── D. search ─────────────────────────────────────────────────────────────

#[test]
fn search_empty_index() {
    let (_dir, project) = init_project("empty-fts");
    sil()
        .current_dir(&project)
        .args(["source", "search", "anything"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No results"));
}

#[test]
fn search_unicode_content_findable_via_ascii_token() {
    // FTS5 default tokenizer does not segment CJK; content may still contain
    // unicode while queries use ASCII tokens that the tokenizer indexes.
    let (_dir, project) = init_project("uni-fts");
    fs::write(
        project.join("sources/u.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/u.pdf"])
        .env("SIL_MARKER_STUB", "注意力机制 selfattentiontoken αβγ café")
        .assert()
        .success();
    sil()
        .current_dir(&project)
        .args(["source", "search", "selfattentiontoken"])
        .assert()
        .success()
        .stdout(predicates::str::contains("u.pdf"));
}

// ── E. git / F. safety ────────────────────────────────────────────────────

#[test]
fn status_reflects_dirty_and_cleanish() {
    let (_dir, project) = init_project("dirty");
    // Uncommitted layout after init
    sil()
        .current_dir(&project)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("uncommitted").or(predicates::str::contains("change")));
}

#[test]
fn reparse_same_pdf_fails_idempotently() {
    let (_dir, project) = init_project("reparse");
    fs::write(
        project.join("sources/r.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/r.pdf"])
        .env("SIL_MARKER_STUB", "first")
        .assert()
        .success();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/r.pdf"])
        .env("SIL_MARKER_STUB", "second")
        .assert()
        .failure()
        .stderr(predicates::str::contains("already parsed"));
}

#[test]
fn second_init_fails_clearly() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("twice");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(
            predicates::str::contains("already")
                .or(predicates::str::contains("exists"))
                .or(predicates::str::contains("sil project")),
        );
}
