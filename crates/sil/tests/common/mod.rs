//! Shared helpers for `sil` end-to-end tests.
//!
//! Colors/progress are forced off so output stays deterministic.
//!
//! Each integration test binary only uses a subset of these helpers, so unused
//! items are expected and must not fail `-D warnings` / clippy.

#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::TempDir;

/// Build a `sil` command with non-interactive, colorless env.
pub fn sil() -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("sil");
    cmd.env("SIL_NO_COLOR", "1")
        .env("SIL_NONINTERACTIVE", "1")
        .env("NO_COLOR", "1")
        .env("SIL_MARKER_STUB", "transformer attention mechanism for testing");
    cmd
}

/// Create a temp project via real `sil init`.
pub fn init_project(name: &str) -> (TempDir, PathBuf) {
    let dir = tempfile::tempdir().expect("tempdir");
    let project = dir.path().join(name);
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();
    (dir, project)
}

/// Assert that a file contains a substring.
pub fn assert_file_contains(path: &Path, needle: &str) {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        text.contains(needle),
        "expected {:?} to contain {:?}\n--- content ---\n{text}",
        path,
        needle
    );
}

/// Run a git command in a project directory.
pub fn git(project: &Path, args: &[&str]) -> std::process::Output {
    Command::new("git")
        .args(args)
        .current_dir(project)
        .output()
        .expect("git")
}

/// Commit all with an optional Sci-Action trailer body.
pub fn git_commit_all(project: &Path, message: &str) {
    let _ = git(project, &["add", "-A"]);
    let status = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(project)
        .status()
        .expect("git commit");
    assert!(status.success(), "git commit failed");
}
