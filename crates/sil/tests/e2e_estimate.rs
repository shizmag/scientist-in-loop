mod common;

use common::{init_project, sil};

#[test]
fn estimate_command_quick_mode() {
    let (_dir, project) = init_project("estimate_quick_test");

    let output = sil()
        .current_dir(&project)
        .args(["paper", "estimate", "--mode", "quick"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Manuscript estimate"));
}

#[test]
fn estimate_command_json_output() {
    let (_dir, project) = init_project("estimate_json_test");

    let output = sil()
        .current_dir(&project)
        .args(["paper", "estimate", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert!(json.get("overall_score").is_some());
    assert!(json.get("mode").is_some());
}

#[test]
fn estimate_command_write_creates_review() {
    let (_dir, project) = init_project("estimate_write_test");

    let output = sil()
        .current_dir(&project)
        .args(["paper", "estimate", "--write"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Wrote estimate report to"));

    let reviews_dir = project.join(".sil").join("reviews");
    assert!(reviews_dir.exists());

    let entries: Vec<_> = std::fs::read_dir(&reviews_dir)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|s| s.to_str()) == Some("md"))
        .collect();

    assert!(
        !entries.is_empty(),
        "expected review markdown file under .sil/reviews/"
    );
}
