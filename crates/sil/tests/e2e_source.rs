//! E2E: `sil source fetch` error surfacing (no live network required for failure path).

mod common;

use common::{init_project, sil};

#[test]
fn source_fetch_surfaces_download_failure() {
    let (_dir, project) = init_project("fetchfail");

    // example.com returns non-PDF / 404 for this path — helper must fail cleanly.
    sil()
        .current_dir(&project)
        .args([
            "source",
            "fetch",
            "https://example.com/not-a-real-pdf-for-sil-tests",
            "--no-parse",
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("download failed"));
}

#[test]
fn source_help_lists_fetch() {
    sil()
        .args(["source", "--help"])
        .assert()
        .success()
        .stdout(predicates::str::contains("fetch"));
}
