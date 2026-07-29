//! End-to-end tests for `sil init` and core command paths.
//!
//! Runs the real `sil` binary against temporary directories.
//! Colors/progress are disabled via env for deterministic output.

use std::fs;
use std::path::Path;
use std::process::Command;

use assert_cmd::cargo::cargo_bin_cmd;
use tempfile::tempdir;

fn sil() -> assert_cmd::Command {
    let mut cmd = cargo_bin_cmd!("sil");
    cmd.env("SIL_NO_COLOR", "1")
        .env("SIL_NONINTERACTIVE", "1")
        .env("NO_COLOR", "1")
        .env("SIL_MARKER_STUB", "transformer attention mechanism for testing");
    cmd
}

fn assert_file_contains(path: &Path, needle: &str) {
    let text = fs::read_to_string(path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()));
    assert!(
        text.contains(needle),
        "expected {:?} to contain {:?}\n--- content ---\n{text}",
        path,
        needle
    );
}

#[test]
fn init_creates_exact_layout_and_readmes() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("my-paper");

    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("Commit proposal"))
        .stdout(predicates::str::contains("Sci-Action: init"));

    // Directory tree
    for rel in [
        ".sil/config.yaml",
        ".sil/structure.yaml",
        ".sil/structure.example.yaml",
        ".sil/db.sqlite",
        ".sil/skills/SYSTEM.md",
        ".sil/skills/paper.md",
        ".sil/skills/agent-code.md",
        "paper_draft.tex",
        "paper.tex",
        "references.bib",
        "sources",
        "data/README.md",
        "figures/plots/README.md",
        "figures/images/README.md",
        "agent/README.md",
        "README.md",
        ".git",
    ] {
        let p = project.join(rel);
        assert!(p.exists(), "missing {rel}");
    }

    // Skill contents (exact required phrases)
    assert_file_contains(
        &project.join(".sil/skills/SYSTEM.md"),
        "SYSTEM RULES FOR THIS PROJECT",
    );
    assert_file_contains(
        &project.join(".sil/skills/SYSTEM.md"),
        "Never auto-commit",
    );
    assert_file_contains(
        &project.join(".sil/skills/paper.md"),
        "structure.yaml",
    );
    assert_file_contains(
        &project.join(".sil/skills/agent-code.md"),
        "agent/README.md",
    );

    // Folder README headings
    assert_file_contains(&project.join("data/README.md"), "# Data");
    assert_file_contains(
        &project.join("figures/plots/README.md"),
        "# Generated Plots",
    );
    assert_file_contains(
        &project.join("figures/images/README.md"),
        "# External Images",
    );
    assert_file_contains(&project.join("agent/README.md"), "# Agent-written code");

    // Non-empty READMEs
    for rel in [
        "data/README.md",
        "figures/plots/README.md",
        "figures/images/README.md",
        "agent/README.md",
        "README.md",
    ] {
        let meta = fs::metadata(project.join(rel)).unwrap();
        assert!(meta.len() > 20, "{rel} too short");
    }

    // Config schema keys
    let cfg = fs::read_to_string(project.join(".sil/config.yaml")).unwrap();
    assert!(cfg.contains("stage:"));
    assert!(cfg.contains("engine: tectonic"));
    assert!(cfg.contains("engine: marker"));

    // Example structure ships with attention paper content
    assert_file_contains(
        &project.join(".sil/structure.example.yaml"),
        "Attention Is All You Need",
    );

    // Re-init should fail
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .failure();
}

/// Source literature PDFs and figure PDFs must be trackable; only root build PDFs ignored.
#[test]
fn init_gitignore_allows_source_and_figure_pdfs() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("pdf-git");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    let gitignore = fs::read_to_string(project.join(".gitignore")).unwrap();
    assert!(
        gitignore.contains("/*.pdf") || gitignore.contains("!sources/"),
        "gitignore should scope PDF ignore to root and un-ignore sources/figures:\n{gitignore}"
    );
    assert!(
        !gitignore.lines().any(|l| l.trim() == "*.pdf"),
        "blanket *.pdf must not ignore sources/figures PDFs:\n{gitignore}"
    );

    // Real fixture paths git will evaluate
    fs::write(project.join("sources/paper.pdf"), sil_parse::minimal_pdf_bytes()).unwrap();
    fs::write(
        project.join("figures/plots/fig1.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    fs::write(
        project.join("figures/images/photo.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    fs::write(project.join("paper_draft.pdf"), sil_parse::minimal_pdf_bytes()).unwrap();

    // git check-ignore -q: exit 0 = ignored, 1 = not ignored
    let check = |rel: &str| -> bool {
        let status = Command::new("git")
            .args(["-C", project.to_str().unwrap(), "check-ignore", "-q", rel])
            .status()
            .expect("git check-ignore");
        status.success()
    };

    assert!(
        !check("sources/paper.pdf"),
        "sources/paper.pdf must NOT be gitignored"
    );
    assert!(
        !check("figures/plots/fig1.pdf"),
        "figures/plots/fig1.pdf must NOT be gitignored"
    );
    assert!(
        !check("figures/images/photo.pdf"),
        "figures/images/photo.pdf must NOT be gitignored"
    );
    assert!(
        check("paper_draft.pdf"),
        "root paper_draft.pdf SHOULD be gitignored as a build artifact"
    );

    // git add -A dry path: source PDF must become staged; root PDF must not
    Command::new("git")
        .args(["-C", project.to_str().unwrap(), "add", "-A"])
        .status()
        .unwrap();
    let staged = Command::new("git")
        .args(["-C", project.to_str().unwrap(), "diff", "--cached", "--name-only"])
        .output()
        .unwrap();
    let staged = String::from_utf8_lossy(&staged.stdout);
    assert!(
        staged.contains("sources/paper.pdf"),
        "sources/paper.pdf should be stageable after git add -A:\n{staged}"
    );
    assert!(
        staged.contains("figures/plots/fig1.pdf"),
        "figures/plots/fig1.pdf should be stageable:\n{staged}"
    );
    assert!(
        !staged.contains("paper_draft.pdf"),
        "root paper_draft.pdf must not be staged:\n{staged}"
    );
}

#[test]
fn status_reflects_project() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("stat");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    sil()
        .current_dir(&project)
        .arg("status")
        .assert()
        .success()
        .stdout(predicates::str::contains("stage:"))
        .stdout(predicates::str::contains("draft"))
        .stdout(predicates::str::contains("database:"))
        .stdout(predicates::str::contains("sections"));
}

#[test]
fn parse_and_search_e2e() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("parseproj");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    // Fixture PDF with magic bytes
    let pdf = project.join("sources/attention.pdf");
    fs::write(&pdf, sil_parse::minimal_pdf_bytes()).unwrap();

    sil()
        .current_dir(&project)
        .args(["parse", "sources/attention.pdf"])
        .env("SIL_MARKER_STUB", "transformer multi-head self-attention mechanism")
        .assert()
        .success()
        .stdout(predicates::str::contains("Parsed"))
        .stdout(predicates::str::contains("Sci-Action: parse-pdf"));

    sil()
        .current_dir(&project)
        .args(["search", "transformer"])
        .assert()
        .success()
        .stdout(predicates::str::contains("attention.pdf"));

    // Already parsed rejects
    sil()
        .current_dir(&project)
        .args(["parse", "sources/attention.pdf"])
        .env("SIL_MARKER_STUB", "x")
        .assert()
        .failure()
        .stderr(predicates::str::contains("already parsed"));

    // Invalid non-PDF
    let bad = project.join("sources/notes.txt");
    fs::write(&bad, "hello").unwrap();
    sil()
        .current_dir(&project)
        .args(["parse", "sources/notes.txt"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not a PDF"));

    // Missing file
    sil()
        .current_dir(&project)
        .args(["parse", "sources/missing.pdf"])
        .assert()
        .failure()
        .stderr(predicates::str::contains("not found"));
}

#[test]
fn parse_no_args_selects_all_noninteractive() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("multi");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    for name in ["a.pdf", "b.pdf"] {
        fs::write(
            project.join("sources").join(name),
            sil_parse::minimal_pdf_bytes(),
        )
        .unwrap();
    }

    sil()
        .current_dir(&project)
        .arg("parse")
        .env("SIL_MARKER_STUB", "batch parse content unique token xyzzy")
        .assert()
        .success()
        .stdout(predicates::str::contains("PDF"));

    sil()
        .current_dir(&project)
        .args(["search", "xyzzy"])
        .assert()
        .success();
}

#[test]
fn context_default_and_flags() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("ctx");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    // Apply initial commit so log has something optional
    let _ = Command::new("git")
        .args(["-C", project.to_str().unwrap(), "add", "-A"])
        .status();
    let _ = Command::new("git")
        .args([
            "-C",
            project.to_str().unwrap(),
            "commit",
            "-m",
            "Initialize sil project\n\nSci-Action: init\n",
        ])
        .status();

    sil()
        .current_dir(&project)
        .arg("context")
        .assert()
        .success()
        .stdout(predicates::str::contains("SYSTEM RULES FOR THIS PROJECT"))
        .stdout(predicates::str::contains("structure.yaml"))
        .stdout(predicates::str::contains("config.yaml"))
        .stdout(predicates::str::contains("Sources summary"));

    sil()
        .current_dir(&project)
        .args(["context", "--paper", "--agent", "--skill-paper"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Paper content"))
        .stdout(predicates::str::contains("Agent directory"))
        .stdout(predicates::str::contains("Working with the paper"));
}

#[test]
fn log_shows_sci_action() {
    let dir = tempdir().unwrap();
    let project = dir.path().join("logp");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    Command::new("git")
        .args(["-C", project.to_str().unwrap(), "add", "-A"])
        .status()
        .unwrap();
    Command::new("git")
        .args([
            "-C",
            project.to_str().unwrap(),
            "commit",
            "-m",
            "Initialize sil project\n\nSci-Action: init\n",
        ])
        .status()
        .unwrap();

    sil()
        .current_dir(&project)
        .arg("log")
        .assert()
        .success()
        .stdout(predicates::str::contains("init"))
        .stdout(predicates::str::contains("Initialize"));
}

#[test]
fn help_lists_all_commands() {
    let out = sil().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    for cmd in [
        "init", "status", "parse", "source", "search", "build", "log", "context",
    ] {
        assert!(stdout.contains(cmd), "help missing {cmd}:\n{stdout}");
    }
}
