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
        .args(["parse", "sources/one.pdf"])
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
