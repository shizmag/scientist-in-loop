use assert_cmd::Command;
use predicates::prelude::*;
use tempfile::tempdir;

#[test]
fn test_sil_doctor_manuscript_health_e2e() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("my-paper");

    // sil init
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("init").arg("my-paper").current_dir(dir.path()).assert().success();

    // sil doctor inside project
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("doctor").current_dir(&project).assert().success()
        .stdout(predicate::str::contains("manuscript health: citations"))
        .stdout(predicate::str::contains("manuscript health: word count"));

    // sil doctor --json
    let mut cmd = Command::cargo_bin("sil").unwrap();
    cmd.arg("doctor").arg("--json").current_dir(&project).assert().success()
        .stdout(predicate::str::contains("manuscript health: citations"))
        .stdout(predicate::str::contains("checks"));
}
