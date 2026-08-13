mod common;

use common::{init_project, sil};

#[test]
fn recent_command_tracks_and_lists_projects() {
    let (_dir, project) = init_project("recent_test");

    // Running sil status in project dir touches recent projects
    sil().current_dir(&project).arg("status").assert().success();

    let output = sil()
        .current_dir(&project)
        .args(["paper", "recent"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Recent sil projects"));
}

#[test]
fn recent_command_json_output() {
    let (_dir, project) = init_project("recent_json_test");

    sil().current_dir(&project).arg("status").assert().success();

    let output = sil()
        .current_dir(&project)
        .args(["paper", "recent", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid json array");
    assert!(json.is_array());
}
