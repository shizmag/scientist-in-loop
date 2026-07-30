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
        ".sil/improvement",
        ".sil/improvement/README.md",
        ".sil/draft_sections",
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
        &project.join(".sil/improvement/README.md"),
        "suggestion_n",
    );

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
        .failure()
        .stderr(predicates::str::contains("already a sil project"));
}

#[test]
fn init_gitignore_ignores_large_artifacts_keeps_sources_and_readmes() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("pdf-git");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    let gitignore = fs::read_to_string(project.join(".gitignore")).unwrap();
    assert!(
        gitignore.contains("# >>> sil-managed") && gitignore.contains("# <<< sil-managed"),
        "gitignore should use sil-managed markers:\n{gitignore}"
    );
    assert!(
        gitignore.contains(".sil/db.sqlite"),
        "db should be ignored:\n{gitignore}"
    );
    assert!(
        gitignore.contains("figures/images/**") && gitignore.contains("figures/plots/**"),
        "figure binaries should be ignored:\n{gitignore}"
    );
    assert!(
        gitignore.contains("data/**"),
        "experiment data should be ignored:\n{gitignore}"
    );
    assert!(
        gitignore.contains("*.pdf"),
        "PDFs should be ignored:\n{gitignore}"
    );

    fs::write(project.join("sources/paper.pdf"), sil_parse::minimal_pdf_bytes()).unwrap();
    fs::write(
        project.join("figures/plots/fig1.pdf"),
        sil_parse::minimal_pdf_bytes(),
    )
    .unwrap();
    fs::write(
        project.join("figures/images/photo.png"),
        b"\x89PNG\r\n\x1a\n",
    )
    .unwrap();
    fs::write(project.join("data/results.csv"), b"a,b\n1,2\n").unwrap();
    fs::write(project.join("paper_draft.pdf"), sil_parse::minimal_pdf_bytes()).unwrap();
    // DB is created by init; ensure it exists for check-ignore
    assert!(project.join(".sil/db.sqlite").exists());

    let check = |rel: &str| -> bool {
        Command::new("git")
            .args(["-C", project.to_str().unwrap(), "check-ignore", "-q", rel])
            .status()
            .expect("git check-ignore")
            .success()
    };

    assert!(check("sources/paper.pdf"), "sources literature PDFs ignored by default");
    assert!(!check("sources/README.md"));
    assert!(!check("figures/plots/README.md"));
    assert!(!check("figures/images/README.md"));
    assert!(!check("data/README.md"));
    assert!(check("figures/plots/fig1.pdf"), "plot binaries ignored by default");
    assert!(check("figures/images/photo.png"), "image binaries ignored by default");
    assert!(check("data/results.csv"), "experiment data ignored by default");
    assert!(check("paper_draft.pdf"), "root build PDF ignored");
    assert!(check(".sil/db.sqlite"), "sqlite db ignored");

    // Improvement proposals and draft section cache must stay trackable
    fs::write(
        project.join(".sil/improvement/suggestion_1"),
        "proposal: tighten abstract\n",
    )
    .unwrap();
    fs::create_dir_all(project.join(".sil/draft_sections")).unwrap();
    fs::write(
        project.join(".sil/draft_sections/01-introduction.tex"),
        "% section body\n",
    )
    .unwrap();
    assert!(
        !check(".sil/improvement/suggestion_1"),
        "improvement proposals must not be gitignored"
    );
    assert!(
        !check(".sil/improvement/README.md"),
        "improvement README must not be gitignored"
    );
    assert!(
        !check(".sil/draft_sections/01-introduction.tex"),
        "draft_sections must not be gitignored"
    );

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
    assert!(!staged.contains("sources/paper.pdf"), "PDFs in sources/ must be gitignored:\n{staged}");
    assert!(staged.contains("sources/README.md"), "sources/README.md must be stageable:\n{staged}");
    assert!(staged.contains("figures/plots/README.md"), "{staged}");
    assert!(
        staged.contains("suggestion_1") || staged.contains("improvement"),
        "improvement proposals should be stageable:\n{staged}"
    );
    assert!(
        staged.contains("01-introduction.tex") || staged.contains("draft_sections"),
        "draft_sections should be stageable:\n{staged}"
    );
    assert!(!staged.contains("fig1.pdf"), "{staged}");
    assert!(!staged.contains("results.csv"), "{staged}");
    assert!(!staged.contains("paper_draft.pdf"), "{staged}");
    assert!(!staged.contains("db.sqlite"), "{staged}");
}

#[test]
fn init_update_refreshes_templates_preserves_user_files() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("upgrade-me");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    // Simulate older / customized project state
    fs::write(
        project.join(".sil/skills/SYSTEM.md"),
        "# OLD SYSTEM\n",
    )
    .unwrap();
    fs::write(
        project.join(".sil/structure.yaml"),
        "title: User Paper\nsections: []\n",
    )
    .unwrap();
    fs::write(
        project.join(".sil/config.yaml"),
        "project:\n  title: Custom\n  stage: prep\npaths:\n  sources: ./sources\n  data: ./data\n  figures: ./figures\n  agent: ./agent\nlatex:\n  engine: pdflatex\n  main: paper_draft.tex\nparsing:\n  engine: marker\n",
    )
    .unwrap();
    fs::write(project.join("paper_draft.tex"), "% user draft\n").unwrap();
    // Custom gitignore rule after managed block
    let mut gi = fs::read_to_string(project.join(".gitignore")).unwrap();
    gi.push_str("\n# my local rule\n*.secret\n");
    // Simulate outdated managed block content
    gi = gi.replace(".sil/db.sqlite", ".sil/db.sqlite\n# old-marker-line");
    fs::write(project.join(".gitignore"), &gi).unwrap();
    // Remove a scaffold file to verify recreation
    fs::remove_file(project.join("agent/README.md")).unwrap();

    sil()
        .current_dir(&project)
        .args(["init", "--update"])
        .assert()
        .success()
        .stdout(predicates::str::contains("Updated sil workspace"))
        .stdout(predicates::str::contains("Sci-Action: update"));

    // Skills refreshed
    let system = fs::read_to_string(project.join(".sil/skills/SYSTEM.md")).unwrap();
    assert!(
        system.contains("SYSTEM RULES FOR THIS PROJECT"),
        "skills should be refreshed:\n{system}"
    );
    assert!(!system.contains("OLD SYSTEM"));

    // User content preserved
    assert_eq!(
        fs::read_to_string(project.join(".sil/structure.yaml")).unwrap(),
        "title: User Paper\nsections: []\n"
    );
    let cfg = fs::read_to_string(project.join(".sil/config.yaml")).unwrap();
    assert!(cfg.contains("title: Custom") && cfg.contains("engine: pdflatex"));
    assert_eq!(
        fs::read_to_string(project.join("paper_draft.tex")).unwrap(),
        "% user draft\n"
    );

    // Missing scaffold recreated
    assert!(project.join("agent/README.md").is_file());

    // Custom gitignore rule preserved; managed content present
    let gi = fs::read_to_string(project.join(".gitignore")).unwrap();
    assert!(gi.contains("*.secret"), "custom rules must survive update:\n{gi}");
    assert!(gi.contains(".sil/db.sqlite"));
    assert!(gi.contains("# >>> sil-managed"));

    // Second update is idempotent enough to succeed
    sil()
        .current_dir(&project)
        .args(["init", "--update"])
        .assert()
        .success();
}

#[test]
fn init_update_fails_outside_project() {
    let dir = tempfile::tempdir().unwrap();
    sil()
        .current_dir(dir.path())
        .args(["init", "--update"])
        .assert()
        .failure();
}

#[test]
fn init_without_update_still_rejects_existing_project() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("twice");
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();
    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicates::str::contains("already a sil project"));
}

#[test]
fn help_lists_all_commands() {
    let out = sil().arg("--help").assert().success();
    let stdout = String::from_utf8_lossy(&out.get_output().stdout);
    for cmd in [
        "init", "status", "parse", "source", "search", "build", "log", "context", "split",
        "propose", "promote", "structure", "cite", "doctor",
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
