//! E2E: `sil init` layout, templates, skills, gitignore.

mod common;

use std::fs;
use std::process::Command;

use common::{assert_file_contains, sil};

#[test]
fn init_creates_exact_layout_and_readmes() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("my-paper");

    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicates::str::contains("Commit proposal"))
        .stdout(predicates::str::contains("Sci-Action: init"));

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
        assert!(project.join(rel).exists(), "missing {rel}");
    }

    assert_file_contains(
        &project.join(".sil/skills/SYSTEM.md"),
        "SYSTEM RULES FOR THIS PROJECT",
    );
    assert_file_contains(&project.join(".sil/skills/SYSTEM.md"), "Never auto-commit");
    assert_file_contains(&project.join(".sil/skills/paper.md"), "structure.yaml");
    assert_file_contains(&project.join(".sil/skills/agent-code.md"), "agent/README.md");

    assert_file_contains(&project.join("data/README.md"), "# Data");
    assert_file_contains(&project.join("figures/plots/README.md"), "# Generated Plots");
    assert_file_contains(&project.join("figures/images/README.md"), "# External Images");
    assert_file_contains(&project.join("agent/README.md"), "# Agent-written code");

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

    let cfg = fs::read_to_string(project.join(".sil/config.yaml")).unwrap();
    assert!(cfg.contains("stage:"));
    assert!(cfg.contains("engine: tectonic"));
    assert!(cfg.contains("engine: marker"));

    assert_file_contains(
        &project.join(".sil/structure.example.yaml"),
        "Attention Is All You Need",
    );

    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .failure();
}

#[test]
fn init_gitignore_allows_source_and_figure_pdfs() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("pdf-git");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    let gitignore = fs::read_to_string(project.join(".gitignore")).unwrap();
    assert!(
        gitignore.contains("/*.pdf") || gitignore.contains("!sources/"),
        "gitignore should scope PDF ignore to root:\n{gitignore}"
    );
    assert!(
        !gitignore.lines().any(|l| l.trim() == "*.pdf"),
        "blanket *.pdf must not ignore sources/figures:\n{gitignore}"
    );

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

    let check = |rel: &str| -> bool {
        Command::new("git")
            .args(["-C", project.to_str().unwrap(), "check-ignore", "-q", rel])
            .status()
            .expect("git check-ignore")
            .success()
    };

    assert!(!check("sources/paper.pdf"));
    assert!(!check("figures/plots/fig1.pdf"));
    assert!(!check("figures/images/photo.pdf"));
    assert!(check("paper_draft.pdf"));

    Command::new("git")
        .args(["-C", project.to_str().unwrap(), "add", "-A"])
        .status()
        .unwrap();
    let staged = Command::new("git")
        .args([
            "-C",
            project.to_str().unwrap(),
            "diff",
            "--cached",
            "--name-only",
        ])
        .output()
        .unwrap();
    let staged = String::from_utf8_lossy(&staged.stdout);
    assert!(staged.contains("sources/paper.pdf"), "{staged}");
    assert!(staged.contains("figures/plots/fig1.pdf"), "{staged}");
    assert!(!staged.contains("paper_draft.pdf"), "{staged}");
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

#[test]
fn init_skills_contain_loading_rules_and_goal_phrases() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("skills");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    let system = std::fs::read_to_string(project.join(".sil/skills/SYSTEM.md")).unwrap();
    assert!(system.contains("Skill loading rules") || system.contains("Always read this SYSTEM.md"));
    assert!(system.contains("sources/"));
    assert!(system.contains("Never auto-commit"));

    let paper = std::fs::read_to_string(project.join(".sil/skills/paper.md")).unwrap();
    assert!(paper.contains("completion"));
    assert!(paper.contains("paper_draft.tex"));

    let agent = std::fs::read_to_string(project.join(".sil/skills/agent-code.md")).unwrap();
    assert!(agent.contains("figures/plots/"));
    assert!(agent.contains("data/"));
}

#[test]
fn init_in_cwd_when_name_omitted() {
    let dir = tempfile::tempdir().unwrap();
    sil()
        .current_dir(dir.path())
        .arg("init")
        .assert()
        .success();
    assert!(dir.path().join(".sil/config.yaml").exists());
    assert!(dir.path().join("sources").is_dir());
}
