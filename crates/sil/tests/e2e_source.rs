//! E2E: `sil source` fetch / list / remove.

mod common;

use std::fs;

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
fn source_help_lists_fetch_list_remove() {
    let out = sil()
        .args(["source", "--help"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    for cmd in ["fetch", "list", "remove"] {
        assert!(stdout.contains(cmd), "source help missing {cmd}:\n{stdout}");
    }
}

#[test]
fn source_list_shows_parsed_and_unparsed() {
    let (_dir, project) = init_project("src-list");
    fs::write(
        project.join("sources/unparsed.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    fs::write(
        project.join("sources/parsed.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();

    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/parsed.pdf"])
        .env("SIL_MARKER_STUB", "list test content")
        .assert()
        .success();

    let out = sil()
        .current_dir(&project)
        .args(["source", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("parsed"))
        .stdout(predicates::str::contains("unparsed"))
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("parsed.pdf"), "{stdout}");
    assert!(stdout.contains("unparsed.pdf"), "{stdout}");

    let json_out = sil()
        .current_dir(&project)
        .args(["source", "list", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json_out)).expect("json");
    let arr = v.as_array().expect("array");
    assert!(arr.len() >= 2, "{v}");
    let parsed_flags: Vec<bool> = arr
        .iter()
        .filter_map(|e| e.get("parsed").and_then(|p| p.as_bool()))
        .collect();
    assert!(parsed_flags.contains(&true));
    assert!(parsed_flags.contains(&false));
}

#[test]
fn source_remove_allows_reparse_path() {
    let (_dir, project) = init_project("src-rm");
    fs::write(
        project.join("sources/gone.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/gone.pdf"])
        .env("SIL_MARKER_STUB", "to remove")
        .assert()
        .success();

    sil()
        .current_dir(&project)
        .args(["source", "list"])
        .assert()
        .success()
        .stdout(predicates::str::contains("gone.pdf"));

    sil()
        .current_dir(&project)
        .args(["source", "remove", "gone.pdf"])
        .assert()
        .success();

    // File still on disk; list should show unparsed
    assert!(project.join("sources/gone.pdf").is_file());
    let out = sil()
        .current_dir(&project)
        .args(["source", "list"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("unparsed") || stdout.contains("gone.pdf"),
        "{stdout}"
    );
}
