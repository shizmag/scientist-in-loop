mod common;

use common::{init_project, sil};

#[test]
fn pack_command_default_output() {
    let (_dir, project) = init_project("pack_default_test");

    let output = sil()
        .current_dir(&project)
        .args(["paper", "pack"])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("Created reproducible paper pack"));
    assert!(project.join("paper_pack.zip").exists());
}

#[test]
fn pack_command_custom_filename() {
    let (_dir, project) = init_project("pack_custom_test");

    let custom_output = project.join("my_bundle.zip");

    let output = sil()
        .current_dir(&project)
        .args(["paper", "pack", "-o", custom_output.to_str().unwrap()])
        .assert()
        .success()
        .get_output()
        .stdout
        .clone();

    let stdout = String::from_utf8_lossy(&output);
    assert!(stdout.contains("my_bundle.zip"));
    assert!(custom_output.exists());
}
