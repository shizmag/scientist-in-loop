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
