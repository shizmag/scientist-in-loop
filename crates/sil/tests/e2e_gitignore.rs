//! E2E: `sil init` gitignore rules and `sil init --update` block refreshing.

mod common;

use std::fs;
use std::process::Command;

use common::{assert_file_contains, sil};

#[test]
fn gitignore_created_with_managed_markers_and_readmes() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("gi-scaffold");

    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    let gi_path = project.join(".gitignore");
    assert!(gi_path.is_file(), ".gitignore must be created by sil init");

    assert_file_contains(&gi_path, "# >>> sil-managed");
    assert_file_contains(&gi_path, "# <<< sil-managed");

    for readme in [
        "sources/README.md",
        "figures/images/README.md",
        "figures/plots/README.md",
        "data/README.md",
        "agent/README.md",
        "README.md",
    ] {
        assert!(
            project.join(readme).is_file(),
            "missing scaffold README: {readme}"
        );
    }
}

#[test]
fn gitignore_ignores_binary_files_and_tracks_text_metadata() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("gi-rules");

    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    let ignored_files = [
        "sources/BEE-RAG.pdf",
        "sources/paper.PDF",
        "paper_draft.pdf",
        "figures/images/photo.jpg",
        "figures/images/diagram.PNG",
        "figures/plots/chart.png",
        "data/results.csv",
        "data/figure.jpeg",
        "agent/output.webp",
        "random/nested/drawing.svg",
    ];

    for rel in &ignored_files {
        let abs = project.join(rel);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        fs::write(&abs, b"binary contents\n").unwrap();
    }

    let trackable_files = [
        "sources/README.md",
        "figures/images/README.md",
        "figures/plots/README.md",
        "data/README.md",
        "paper_draft.tex",
        "paper.tex",
        "references.bib",
        ".sil/config.yaml",
        ".sil/structure.yaml",
        ".sil/improvement/suggestion_1",
        ".sil/draft_sections/01-introduction.tex",
    ];

    for rel in &trackable_files {
        let abs = project.join(rel);
        if let Some(parent) = abs.parent() {
            fs::create_dir_all(parent).unwrap();
        }
        if !abs.exists() {
            fs::write(&abs, b"text content\n").unwrap();
        }
    }

    let is_git_ignored = |rel: &str| -> bool {
        Command::new("git")
            .args(["-C", project.to_str().unwrap(), "check-ignore", "-q", rel])
            .status()
            .expect("git check-ignore execution")
            .success()
    };

    for file in &ignored_files {
        assert!(
            is_git_ignored(file),
            "expected file to be IGNORED by git check-ignore: {file}"
        );
    }

    for file in &trackable_files {
        assert!(
            !is_git_ignored(file),
            "expected file NOT to be ignored by git check-ignore: {file}"
        );
    }

    // Verify behavior with git add -A and git diff --cached
    let add_status = Command::new("git")
        .args(["-C", project.to_str().unwrap(), "add", "-A"])
        .status()
        .expect("git add -A execution");
    assert!(add_status.success(), "git add -A failed");

    let diff_output = Command::new("git")
        .args([
            "-C",
            project.to_str().unwrap(),
            "diff",
            "--cached",
            "--name-only",
        ])
        .output()
        .expect("git diff --cached execution");
    assert!(diff_output.status.success());

    let staged_output = String::from_utf8_lossy(&diff_output.stdout);
    let staged_lines: Vec<&str> = staged_output.lines().collect();

    for file in &ignored_files {
        assert!(
            !staged_lines.contains(file),
            "ignored binary file should NOT be staged by git add -A: {file}\nStaged files:\n{staged_output}"
        );
    }

    for file in &trackable_files {
        assert!(
            staged_lines.contains(file),
            "trackable file SHOULD be staged by git add -A: {file}\nStaged files:\n{staged_output}"
        );
    }
}

#[test]
fn gitignore_update_refreshes_managed_block_and_preserves_custom_rules() {
    let dir = tempfile::tempdir().unwrap();
    let project = dir.path().join("gi-update-refresh");

    sil()
        .args(["init", project.to_str().unwrap()])
        .assert()
        .success();

    let gi_path = project.join(".gitignore");
    let original = fs::read_to_string(&gi_path).unwrap();

    // Modify managed block with obsolete line and append custom user rules below end marker
    let outdated_marker = "# OUTDATED MANAGED BLOCK LINE";
    let custom_rule_1 = "*.secret_key";
    let custom_rule_2 = "local_notes.txt";

    let modified = original.replace(
        ".sil/db.sqlite",
        &format!(".sil/db.sqlite\n{outdated_marker}"),
    ) + &format!("\n# Custom user section\n{custom_rule_1}\n{custom_rule_2}\n");

    fs::write(&gi_path, &modified).unwrap();

    // Run sil init --update
    sil()
        .current_dir(&project)
        .args(["init", "--update"])
        .assert()
        .success();

    let refreshed = fs::read_to_string(&gi_path).unwrap();

    // Managed markers present
    assert!(refreshed.contains("# >>> sil-managed"));
    assert!(refreshed.contains("# <<< sil-managed"));

    // Managed block was refreshed (outdated line replaced)
    assert!(
        !refreshed.contains(outdated_marker),
        "outdated managed block line should have been removed by init --update:\n{refreshed}"
    );

    // Standard managed entries present
    assert!(refreshed.contains(".sil/db.sqlite"));
    assert!(refreshed.contains("*.pdf"));

    // User rules preserved below # <<< sil-managed
    assert!(
        refreshed.contains(custom_rule_1),
        "custom rule 1 must be preserved after update:\n{refreshed}"
    );
    assert!(
        refreshed.contains(custom_rule_2),
        "custom rule 2 must be preserved after update:\n{refreshed}"
    );

    let end_marker_pos = refreshed.find("# <<< sil-managed").unwrap();
    let rule1_pos = refreshed.find(custom_rule_1).unwrap();
    let rule2_pos = refreshed.find(custom_rule_2).unwrap();

    assert!(
        rule1_pos > end_marker_pos,
        "custom rule 1 must stay below # <<< sil-managed"
    );
    assert!(
        rule2_pos > end_marker_pos,
        "custom rule 2 must stay below # <<< sil-managed"
    );
}
