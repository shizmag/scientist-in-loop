//! E2E: `sil cite` bibliography helper.

mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn cite_from_filename_is_deterministic_and_nonempty() {
    let (_tmp, project) = init_project("cite-fn");
    let out1 = sil()
        .current_dir(&project)
        .args(["source", "cite", "attention_is_all_you_need.pdf"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out2 = sil()
        .current_dir(&project)
        .args(["source", "cite", "attention_is_all_you_need.pdf"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s1 = String::from_utf8_lossy(&out1);
    let s2 = String::from_utf8_lossy(&out2);
    assert_eq!(s1, s2, "cite suggestions must be deterministic");
    assert!(s1.contains("\\cite{"), "{s1}");
    assert!(s1.contains("@article{"), "{s1}");
    assert!(
        s1.contains("attention_is_all_you_need") || s1.contains("cite:"),
        "{s1}"
    );
}

#[test]
fn cite_json_and_append_to_references() {
    let (_tmp, project) = init_project("cite-json");
    let out = sil()
        .current_dir(&project)
        .args(["source", "cite", "transformer attention", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out)).expect("json");
    assert!(
        v.get("cite_key")
            .and_then(|k| k.as_str())
            .map(|s| !s.is_empty())
            == Some(true)
    );
    assert!(
        v.get("cite_command")
            .and_then(|c| c.as_str())
            .map(|s| s.contains("\\cite{"))
            == Some(true)
    );
    assert!(
        v.get("bibtex")
            .and_then(|b| b.as_str())
            .map(|s| s.contains("@article"))
            == Some(true)
    );

    sil()
        .current_dir(&project)
        .args(["source", "cite", "my_source.pdf", "--append"])
        .assert()
        .success();
    let bib = fs::read_to_string(project.join("references.bib")).unwrap();
    assert!(bib.contains("@article{"), "{bib}");
    assert!(bib.contains("my_source"), "{bib}");
}

#[test]
fn cite_from_parsed_source_uses_title() {
    let (_tmp, project) = init_project("cite-src");
    fs::write(
        project.join("sources/paper.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    sil()
        .current_dir(&project)
        .args(["source", "parse", "sources/paper.pdf"])
        .env("SIL_MARKER_STUB", "body")
        .assert()
        .success();

    let out = sil()
        .current_dir(&project)
        .args(["source", "cite", "paper.pdf", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value = serde_json::from_str(&String::from_utf8_lossy(&out)).unwrap();
    let key = v["cite_key"].as_str().expect("cite_key");
    assert!(!key.is_empty(), "{v}");
    let cmd = v["cite_command"].as_str().unwrap();
    assert!(cmd.contains("\\cite{"), "{v}");
    let bib = v["bibtex"].as_str().unwrap();
    assert!(bib.contains("@article{"), "{v}");
    assert!(!bib.trim().is_empty());
}

#[test]
fn help_lists_cite() {
    let out = sil().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    assert!(stdout.contains("cite"), "{stdout}");
}

#[test]
fn cite_promote_removes_tui_added_marker() {
    let (_tmp, project) = init_project("cite-promote");
    let initial_bib = "% [sil: tui-added]\n@article{draftkey,\n  title = {Draft Title},\n  author = {Smith, John}\n}\n";
    fs::write(project.join("references.bib"), initial_bib).unwrap();

    let out = sil()
        .current_dir(&project)
        .args(["source", "cite", "draftkey", "--promote"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Promoted entry 'draftkey'"), "{stdout}");
    assert!(!stdout.contains("Commit proposal"), "{stdout}");

    let bib = fs::read_to_string(project.join("references.bib")).unwrap();
    assert!(!bib.contains("% [sil: tui-added]"), "{bib}");
    assert!(bib.contains("@article{draftkey"), "{bib}");
}

#[test]
fn cite_append_same_paper_preserves_cite_key() {
    let (_tmp, project) = init_project("cite-append-preserve");

    // First append
    let out1 = sil()
        .current_dir(&project)
        .args(["source", "cite", "attention_paper.pdf", "--append"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s1 = String::from_utf8_lossy(&out1);
    assert!(s1.contains("Appended entry to"), "{s1}");
    assert!(!s1.contains("Commit proposal"), "{s1}");

    // Second append (same paper)
    let out2 = sil()
        .current_dir(&project)
        .args(["source", "cite", "attention_paper.pdf", "--append"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let s2 = String::from_utf8_lossy(&out2);
    assert!(s2.contains("Updated existing entry in"), "{s2}");
    assert!(!s2.contains("Commit proposal"), "{s2}");

    let bib = fs::read_to_string(project.join("references.bib")).unwrap();
    assert!(bib.contains("@article{attention_paper"), "{bib}");
}
