mod common;

use common::{init_project, sil};

#[test]
fn rank_draft_text_and_json() {
    let (_dir, project) = init_project("rank_draft_test");

    let output = sil()
        .current_dir(&project)
        .args(["source", "rank-draft"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Draft Cosine Similarity Rankings"));

    let json_output = sil()
        .current_dir(&project)
        .args(["source", "rank-draft", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let json_stdout = String::from_utf8_lossy(&json_output);
    let json: serde_json::Value = serde_json::from_str(&json_stdout).expect("valid json array");
    assert!(json.is_array());
}

#[test]
fn rank_draft_min_score_filter() {
    let (_dir, project) = init_project("rank_draft_filter_test");

    let output = sil()
        .current_dir(&project)
        .args(["source", "rank-draft", "--min-score", "0.8"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Draft Cosine Similarity Rankings"));
}
