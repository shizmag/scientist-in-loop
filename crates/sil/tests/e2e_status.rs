//! E2E: `sil status`.

mod common;

use common::{init_project, sil};

#[test]
fn status_reflects_project() {
    let (_dir, project) = init_project("stat");

    sil()
        .current_dir(&project)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("stage:"))
        .stdout(predicates::str::contains("draft"))
        .stdout(predicates::str::contains("database:"))
        .stdout(predicates::str::contains("sections"));
}

#[test]
fn status_updates_after_parse() {
    let (_dir, project) = init_project("stat2");
    std::fs::write(
        project.join("sources/one.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();

    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/one.pdf"])
        .env("SIL_MARKER_STUB", "status count token")
        .assert()
        .success();

    sil()
        .current_dir(&project)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("1 source"))
        .stdout(predicates::str::contains("1 parsed"));
}

#[test]
fn status_outside_project_fails() {
    let dir = tempfile::tempdir().unwrap();
    sil()
        .current_dir(dir.path())
        .arg("status")
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a sil project"));
}

#[test]
fn status_json_is_valid_with_primary_fields() {
    let (_dir, project) = init_project("stat-json");
    let out = sil()
        .current_dir(&project)
        .args(["status", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    let v: serde_json::Value = serde_json::from_str(&stdout).expect("status --json must be JSON");
    assert!(v.is_object(), "expected object, got {v}");
    assert!(v.get("project").is_some(), "{v}");
    assert!(v.get("stage").is_some(), "{v}");
    assert!(v.get("sources").is_some(), "{v}");
    assert!(v.get("structure").is_some(), "{v}");
    assert!(v.get("git").is_some(), "{v}");
    let sources = v.get("sources").unwrap();
    assert!(sources.get("total").is_some());
    assert!(sources.get("parsed").is_some());
    let structure = v.get("structure").unwrap();
    assert!(structure.get("sections").is_some());
    assert!(
        structure
            .get("sections")
            .and_then(|s| s.as_array())
            .map(|a| !a.is_empty())
            .unwrap_or(false),
        "sections should not be empty theater: {structure}"
    );
}
