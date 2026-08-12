mod common;

use std::fs;

use common::{init_project, sil};

#[test]
fn assets_command_detects_graphics_and_inputs() {
    let (_dir, project) = init_project("assets_test");
    let draft_path = project.join("paper_draft.tex");

    // Add \includegraphics and \input to paper_draft.tex
    let tex = fs::read_to_string(&draft_path).unwrap();
    let updated_tex = format!(
        "{}\n\\input{{sections/custom}}\n\\includegraphics{{figures/plots/sample}}\n",
        tex
    );
    fs::write(&draft_path, updated_tex).unwrap();

    let output = sil()
        .current_dir(&project)
        .args(["paper", "assets"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Paper assets in"));
    assert!(stdout.contains("figures/plots/sample"));
    assert!(stdout.contains("sections/custom"));
}

#[test]
fn assets_command_json_output() {
    let (_dir, project) = init_project("assets_json_test");
    let draft_path = project.join("paper_draft.tex");

    fs::write(
        &draft_path,
        "\\documentclass{article}\n\\begin{document}\n\\includegraphics{fig1}\n\\end{document}",
    )
    .unwrap();

    let output = sil()
        .current_dir(&project)
        .args(["paper", "assets", "--json"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    let json: serde_json::Value = serde_json::from_str(&stdout).expect("valid json");
    assert_eq!(json["total_count"], 1);
    assert_eq!(json["all_found"], false);
    assert_eq!(json["graphics"][0]["path"], "fig1");
}

#[test]
fn assets_command_missing_assets_warns() {
    let (_dir, project) = init_project("assets_missing_test");

    let output = sil()
        .current_dir(&project)
        .args(["paper", "assets"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("No \\includegraphics or \\input assets detected"));
}
