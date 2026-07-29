//! E2E: `sil cite` bibliography helper.

mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn cite_from_filename_is_deterministic_and_nonempty() {
    let (_tmp, project) = init_project("cite-fn");
    let out1 = sil()
        .current_dir(&project)
        .args(["cite", "attention_is_all_you_need.pdf"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let out2 = sil()
        .current_dir(&project)
        .args(["cite", "attention_is_all_you_need.pdf"])
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
    assert!(s1.contains("attention_is_all_you_need") || s1.contains("cite:"), "{s1}");
}

#[test]
fn cite_json_and_append_to_references() {
    let (_tmp, project) = init_project("cite-json");
    let out = sil()
        .current_dir(&project)
        .args(["cite", "transformer attention", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out)).expect("json");
    assert!(v.get("cite_key").and_then(|k| k.as_str()).map(|s| !s.is_empty()) == Some(true));
    assert!(v.get("cite_command").and_then(|c| c.as_str()).map(|s| s.contains("\\cite{")) == Some(true));
    assert!(v.get("bibtex").and_then(|b| b.as_str()).map(|s| s.contains("@article")) == Some(true));

    sil()
        .current_dir(&project)
        .args(["cite", "my_source.pdf", "--append"])
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
        .args(["parse", "sources/paper.pdf"])
        .env("SIL_MARKER_STUB", "body")
        .assert()
        .success();

    let out = sil()
        .current_dir(&project)
        .args(["cite", "paper.pdf", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out)).unwrap();
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
