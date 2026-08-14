//! E2E: `sil doctor` and CI workflow presence.

mod common;

use std::path::PathBuf;

use common::{init_project, sil};

#[test]
fn doctor_reports_project_checks() {
    let (_tmp, project) = init_project("doc-me");
    let out = sil()
        .current_dir(&project)
        .args(["project", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("git") || stdout.contains("sil project"),
        "{stdout}"
    );
    assert!(
        stdout.contains("improvement") || stdout.contains("draft_sections") || stdout.contains("✓"),
        "{stdout}"
    );
}

#[test]
fn doctor_json_has_checks() {
    let (_tmp, project) = init_project("doc-json");
    let out = sil()
        .current_dir(&project)
        .args(["project", "doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out)).expect("doctor --json");
    assert!(
        v.get("checks")
            .and_then(|c| c.as_array())
            .map(|a| !a.is_empty())
            == Some(true)
    );
    assert!(v.get("ok").is_some());

    let checks = v["checks"].as_array().unwrap();
    for c in checks {
        assert!(c.get("name").is_some());
        assert!(c.get("ok").is_some());
        assert!(c.get("detail").is_some());
        // If hint is present, it must be a non-empty string
        if let Some(hint) = c.get("hint") {
            assert!(hint.is_string() && !hint.as_str().unwrap().is_empty());
        }
    }
}

#[test]
fn doctor_outside_project_reports_hint() {
    let tmp = tempfile::tempdir().unwrap();
    let out = sil()
        .current_dir(tmp.path())
        .args(["project", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("sil project"), "{stdout}");
    assert!(stdout.contains("Hint: Run `sil init`"), "{stdout}");

    // Also check JSON format outside project
    let json_out = sil()
        .current_dir(tmp.path())
        .args(["project", "doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json_out)).expect("doctor --json");
    let checks = v["checks"].as_array().expect("checks");
    let proj_check = checks
        .iter()
        .find(|c| c["name"] == "sil project")
        .expect("sil project check");
    assert_eq!(proj_check["ok"], false);
    assert!(
        proj_check["hint"]
            .as_str()
            .unwrap()
            .contains("Run `sil init`")
    );
}

#[test]
fn doctor_missing_sources_shows_hint() {
    let (_tmp, project) = init_project("doc-missing-sources");
    let sources_dir = project.join("sources");
    if sources_dir.exists() {
        std::fs::remove_dir_all(&sources_dir).unwrap();
    }

    let out = sil()
        .current_dir(&project)
        .args(["project", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("sources"), "{stdout}");
    assert!(stdout.contains("Create `sources/` directory"), "{stdout}");

    let json_out = sil()
        .current_dir(&project)
        .args(["project", "doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&json_out)).expect("doctor --json");
    let checks = v["checks"].as_array().expect("checks");
    let sources_check = checks
        .iter()
        .find(|c| c["name"] == "sources")
        .expect("sources check");
    assert_eq!(sources_check["ok"], false);
    assert!(
        sources_check["hint"]
            .as_str()
            .unwrap()
            .contains("Create `sources/` directory")
    );
}

#[test]
fn doctor_reports_bib_coverage_ratio() {
    let (_tmp, project) = init_project("doc-bib-cov");
    let bib_content = "@article{cited_key, title={C}}\n@article{uncited_key, title={U}}\n";
    let tex_content = "\\section{Intro}\nWe cite \\cite{cited_key}.\n";

    std::fs::write(project.join("references.bib"), bib_content).unwrap();
    std::fs::write(project.join("paper_draft.tex"), tex_content).unwrap();

    let out = sil()
        .current_dir(&project)
        .args(["project", "doctor"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(
        stdout.contains("1/2 references mentioned in paper_*.tex"),
        "{stdout}"
    );
}

#[test]
fn ci_workflow_exists_and_runs_tests() {
    // Repo root: crates/sil/tests -> ../../../
    let mut root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    root.pop(); // crates
    root.pop(); // repo root
    let wf = root.join(".github/workflows/ci.yml");
    assert!(wf.is_file(), "missing CI workflow at {}", wf.display());
    let text = std::fs::read_to_string(&wf).unwrap();
    assert!(text.contains("cargo test"), "{text}");
    assert!(
        text.contains("clippy") || text.contains("cargo test --workspace"),
        "{text}"
    );
}

#[test]
fn doctor_repair_db_corrupt_database_recovery() {
    let (_tmp, project) = init_project("doc-repair-corrupt");
    let source_file = project.join("sources/article.md");
    let source_content =
        "# Sample Article\n\nDeep learning methods.\n\nReferences\n1. Author 2024.";
    std::fs::write(&source_file, source_content).expect("write source article");

    // Corrupt db.sqlite with invalid binary payload
    let db_path = project.join(".sil/db.sqlite");
    let corrupt_payload = b"CORRUPTED_SQLITE_GARBAGE_PAYLOAD_TEST_BYTES";
    std::fs::write(&db_path, corrupt_payload).expect("corrupt db.sqlite");

    // Run repair-db
    let out = sil()
        .current_dir(&project)
        .args(["project", "doctor", "--repair-db"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let stdout = String::from_utf8_lossy(&out);
    assert!(stdout.contains("Database Repair Report"), "{stdout}");
    assert!(stdout.contains("Backed up corrupt database"), "{stdout}");
    assert!(stdout.contains("Fresh database initialized"), "{stdout}");

    // Verify backup file exists in .sil/ and matches the corrupt payload
    let mut backup_found = false;
    for entry in std::fs::read_dir(project.join(".sil")).expect("read .sil") {
        let entry = entry.expect("entry");
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("db.sqlite.corrupt-") {
            backup_found = true;
            let backup_bytes = std::fs::read(entry.path()).expect("read backup");
            assert_eq!(backup_bytes, corrupt_payload);
            break;
        }
    }
    assert!(
        backup_found,
        "expected backup file db.sqlite.corrupt-* to exist"
    );

    // Verify sources/ file remains intact and untouched
    assert!(source_file.is_file());
    assert_eq!(
        std::fs::read_to_string(&source_file).expect("read source"),
        source_content
    );

    // Verify new database is valid and doctor checks pass
    let doc_out = sil()
        .current_dir(&project)
        .args(["project", "doctor", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&doc_out)).expect("doctor --json");
    let checks = v["checks"].as_array().expect("checks");
    let db_check = checks
        .iter()
        .find(|c| c["name"] == "sqlite integrity")
        .expect("sqlite integrity check");
    assert_eq!(db_check["ok"], true);
    assert_eq!(db_check["detail"], "ok");
}

#[test]
fn doctor_repair_db_missing_sources_returns_clean_error() {
    let (_tmp, project) = init_project("doc-repair-no-sources");
    let sources_dir = project.join("sources");
    if sources_dir.exists() {
        std::fs::remove_dir_all(&sources_dir).expect("remove sources/");
    }

    let out = sil()
        .current_dir(&project)
        .args(["project", "doctor", "--repair-db"])
        .assert()
        .failure()
        .get_output()
        .clone();
    let combined = format!(
        "{}\n{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        combined.contains("sources/ directory not found"),
        "expected missing sources error: {combined}"
    );

    // Verify no backup dance was executed that would create corrupt backups or delete files
    for entry in std::fs::read_dir(project.join(".sil")).expect("read .sil") {
        let entry = entry.expect("entry");
        let name = entry.file_name().to_string_lossy().to_string();
        assert!(
            !name.starts_with("db.sqlite.corrupt-"),
            "no backup should be created when sources/ is missing"
        );
    }
}

#[test]
fn doctor_repair_db_json_format() {
    let (_tmp, project) = init_project("doc-repair-json");
    let source_file = project.join("sources/notes.txt");
    std::fs::write(&source_file, "Some plain text notes.").expect("write notes.txt");

    let out = sil()
        .current_dir(&project)
        .args(["project", "doctor", "--repair-db", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();
    let v: serde_json::Value =
        serde_json::from_str(&String::from_utf8_lossy(&out)).expect("json parse repair report");
    assert!(v.get("sources_scanned").is_some());
    assert!(v.get("sources_reparsed").is_some());
    assert!(v.get("sources_failed").is_some());
    assert!(v.get("outcomes").is_some());
    assert_eq!(v["sources_scanned"], 1);
    assert_eq!(v["sources_reparsed"], 1);
    assert_eq!(v["sources_failed"], 0);
}

#[test]
fn doctor_repair_db_never_deletes_sources_directory() {
    let (_tmp, project) = init_project("doc-repair-preserve");
    let f1 = project.join("sources/doc1.md");
    let f2 = project.join("sources/doc2.txt");
    std::fs::write(&f1, "# Doc 1").expect("write doc1");
    std::fs::write(&f2, "Doc 2 text").expect("write doc2");

    sil()
        .current_dir(&project)
        .args(["project", "doctor", "--repair-db"])
        .assert()
        .success();

    assert!(f1.is_file(), "doc1.md must not be deleted");
    assert!(f2.is_file(), "doc2.txt must not be deleted");
    assert_eq!(std::fs::read_to_string(&f1).unwrap(), "# Doc 1");
    assert_eq!(std::fs::read_to_string(&f2).unwrap(), "Doc 2 text");
}
