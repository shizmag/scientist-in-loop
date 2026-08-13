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
