//! E2E: `sil parse` (path mode, batch noninteractive, validation).

mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn parse_path_mode_and_validation() {
    let (_dir, project) = init_project("parseproj");

    let pdf = project.join("sources/attention.pdf");
    fs::write(&pdf, sil_parse::minimal_pdf_bytes()).unwrap();

    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/attention.pdf"])
        .env(
            "SIL_MARKER_STUB",
            "transformer multi-head self-attention mechanism",
        )
        .assert()
        .success()
        .stdout(predicates::str::contains("Parsed"))
        .stdout(predicates::str::contains("Sci-Action: parse-pdf"));

    // Already parsed rejects
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/attention.pdf"])
        .env("SIL_MARKER_STUB", "x")
        .assert()
        .failure()
        .stderr(predicates::str::contains("already parsed"));

    // Invalid / unsupported format
    fs::write(project.join("sources/notes.unsupported"), "hello").unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/notes.unsupported"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("unsupported format"));

    // Missing file
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/missing.pdf"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn parse_no_args_selects_all_noninteractive() {
    let (_dir, project) = init_project("multi");

    for name in ["a.pdf", "b.pdf"] {
        fs::write(
            project.join("sources").join(name),
            sil_parse::minimal_pdf_bytes(),
        )
        .unwrap();
    }

    sil()
        .current_dir(&project)
        .args(["source", "parse"])
        .env("SIL_MARKER_STUB", "batch parse content unique token xyzzy")
        .assert()
        .success()
        .stdout(predicates::str::contains("PDF"));
}

#[test]
fn parse_nothing_when_sources_empty() {
    let (_dir, project) = init_project("empty-src");

    sil()
        .current_dir(&project)
        .args(["source", "parse"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Nothing to parse"));
}

#[test]
fn parse_proposes_trailer_for_batch() {
    let (_dir, project) = init_project("batch-trailer");
    for name in ["x.pdf", "y.pdf"] {
        std::fs::write(
            project.join("sources").join(name),
            sil_parse::minimal_pdf_bytes(),
        )
        .unwrap();
    }

    sil()
        .current_dir(&project)
        .args(["source", "parse"])
        .env("SIL_MARKER_STUB", "batch trailer content")
        .assert()
        .success()
        .stdout(predicates::str::contains("Sci-Action: parse-pdf"));
}
