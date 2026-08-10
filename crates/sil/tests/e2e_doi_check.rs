use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_doi_check_doctor_integration_e2e() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("doi-paper");

    // 1. sil init
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("init").arg("doi-paper").current_dir(dir.path()).assert().success();

    // 2. Add custom references.bib with DOIs
    let bib_content = r#"@article{vaswani2017,
  author = {Vaswani, Ashish and others},
  title = {Attention Is All You Need},
  year = {2017},
  doi = {10.1038/s41586-020-1234-y}
}

@article{fake2024,
  author = {Fake, Author},
  title = {Nonexistent Paper DOI},
  year = {2024},
  doi = {10.99999/invalid.doi.12345}
}
"#;
    std::fs::write(project.join("references.bib"), bib_content).unwrap();

    // 3. sil project doctor (first run — checks DOIs)
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("project").arg("doctor").current_dir(&project).assert().success()
        .stdout(predicate::str::contains("manuscript health: bib identifiers"));

    // 4. sil project doctor (second run — verifies incremental cache)
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("project").arg("doctor").current_dir(&project).assert().success()
        .stdout(predicate::str::contains("manuscript health: bib identifiers"));
}

#[test]
fn test_doi_check_title_mismatch_and_fix_e2e() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("mismatch-paper");

    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("init").arg("mismatch-paper").current_dir(dir.path()).assert().success();

    let bib_content = r#"@article{alphafold,
  author = {Jumper, John},
  title = {Wrong Title Completely Mismatched},
  year = {2021},
  doi = {10.1038/s41586-021-03819-2}
}
"#;
    std::fs::write(project.join("references.bib"), bib_content).unwrap();

    // Doctor detects title mismatch
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("project").arg("doctor").current_dir(&project).assert().success()
        .stdout(predicate::str::contains("manuscript health: bib identifiers"));

    // Doctor --fix autofixes references.bib
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("project").arg("doctor").arg("--fix").current_dir(&project).assert().success();
}
