mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn todo_command_parses_idea_blocks() {
    let (_dir, project) = init_project("todo_cmd_test");

    let draft_path = project.join("paper_draft.tex");
    let content = fs::read_to_string(&draft_path).unwrap();
    let updated_content = format!(
        "{}\n% # -- X -- #\n% TODO: benchmark performance against baselines\n% # -- X -- #\n",
        content
    );
    fs::write(&draft_path, updated_content).unwrap();

    let output = sil()
        .current_dir(&project)
        .args(["paper", "todo"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Active `# -- X -- #` Idea & TODO Blocks"));
    assert!(stdout.contains("benchmark performance against baselines"));
}

#[test]
fn todo_command_json_output() {
    let (_dir, project) = init_project("todo_json_test");

    let draft_path = project.join("paper_draft.tex");
    let content = fs::read_to_string(&draft_path).unwrap();
    let updated_content = format!(
        "{}\n% # -- X -- #\n% TODO: ablation study on learning rate\n% # -- X -- #\n",
        content
    );
    fs::write(&draft_path, updated_content).unwrap();

    let output = sil()
        .current_dir(&project)
        .args(["paper", "todo", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid json array");
    assert!(json.is_array());
    assert!(!json.as_array().unwrap().is_empty());
}
