//! E2E: `sil search` FTS5.

mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn search_returns_parsed_content() {
    let (_dir, project) = init_project("searchproj");

    fs::write(
        project.join("sources/attention.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();

    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/attention.pdf"])
        .env(
            "SIL_MARKER_STUB",
            "transformer multi-head self-attention mechanism",
        )
        .assert()
        .success();

    sil()
        .current_dir(&project)
        .args(["source", "search", "transformer"])
        .assert()
        .success()
        .stdout(predicates::str::contains("attention.pdf"));

    sil()
        .current_dir(&project)
        .args(["source", "search", "zzznomatchtoken"])
        .assert()
        .success()
        .stdout(predicates::str::contains("No results"));
}
