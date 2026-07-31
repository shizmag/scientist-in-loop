//! `sil init` / `sil init --update` — create or upgrade a project workspace.

use std::fs;

use anyhow::{Context, Result, bail};
use camino::Utf8Path;

use sil_core::{ProjectPaths, SciAction, SilUi, paths::rel};
use sil_db::SilDb;
use sil_git::{CommitProposal, init_repo};
use sil_latex::write_draft_sections_from_file;

use crate::templates;

/// Create a new sil project at `target`.
pub fn init_project(target: &Utf8Path, ui: &dyn SilUi) -> Result<()> {
    if target.join(rel::CONFIG).is_file() {
        bail!(
            "already a sil project: {} exists\n  tip: run `sil init --update` to refresh templates",
            target.join(rel::CONFIG)
        );
    }

    let mut spinner = ui.spinner(&format!("Initialising project at {target}"));

    fs::create_dir_all(target.as_str()).with_context(|| format!("create project root {target}"))?;

    let paths = ProjectPaths::new(target);

    ensure_layout(target)?;

    spinner.set_message("Writing templates…");

    // Core config / structure (new projects only)
    write(target, rel::CONFIG, templates::CONFIG_YAML)?;
    write(target, rel::STRUCTURE, templates::STRUCTURE_YAML)?;
    write(
        target,
        ".sil/structure.example.yaml",
        templates::STRUCTURE_EXAMPLE_YAML,
    )?;

    // Skills
    write_skills(target)?;

    // Paper stubs
    write(target, rel::PAPER_DRAFT, templates::PAPER_DRAFT_TEX)?;
    write(target, rel::PAPER_FINAL, templates::PAPER_FINAL_TEX)?;
    write(target, rel::REFERENCES, templates::REFERENCES_BIB)?;

    // Folder READMEs + project README
    write_scaffold_readmes(target, /*overwrite*/ true)?;
    write(target, rel::README, templates::PROJECT_README)?;

    // .gitignore
    write(target, ".gitignore", templates::GITIGNORE)?;

    spinner.set_message("Writing draft section cache…");
    let _ = write_initial_draft_sections(target);

    spinner.set_message("Creating SQLite database…");
    SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("database: {e}"))?;

    spinner.set_message("Initialising git…");
    init_repo(target).map_err(|e| anyhow::anyhow!("{e}"))?;

    spinner.finish_success(&format!("Project ready at {target}"));

    ui.success("Created sil workspace");
    ui.muted(&format!("  config:    {}", paths.config()));
    ui.muted(&format!("  structure: {}", paths.structure()));
    ui.muted(&format!("  database:  {}", paths.db()));
    ui.muted(&format!("  skills:    {}", paths.skills_dir()));

    let proposal = CommitProposal::new("Initialize sil project", SciAction::Init)
        .with_body("Created directory layout, templates, SQLite+FTS5 database, and skills.");
    print_proposal(ui, &proposal);

    Ok(())
}

/// Upgrade an existing sil project to the current binary's templates.
///
/// **Refreshed (always):** skills, `structure.example.yaml`, sil-managed `.gitignore` block.
/// **Created if missing:** layout dirs, folder READMEs, paper stubs, config/structure.
/// **Never overwritten if present:** `config.yaml`, `structure.yaml`, manuscripts,
/// bibliography, project `README.md`, user custom gitignore rules outside the managed block.
pub fn update_project(target: &Utf8Path, ui: &dyn SilUi) -> Result<()> {
    let paths = ProjectPaths::new(target);
    if !paths.is_project() {
        bail!(
            "not a sil project (missing {}):\n  run `sil init` first, or pass the project path",
            paths.config()
        );
    }

    let mut spinner = ui.spinner(&format!("Updating sil project at {target}"));
    let mut changes: Vec<String> = Vec::new();

    ensure_layout(target)?;
    changes.push("ensured directory layout".into());

    // Managed templates — always refresh
    spinner.set_message("Refreshing skills…");
    write_skills(target)?;
    changes.push("refreshed .sil/skills/".into());

    write(
        target,
        ".sil/structure.example.yaml",
        templates::STRUCTURE_EXAMPLE_YAML,
    )?;
    changes.push("refreshed .sil/structure.example.yaml".into());

    spinner.set_message("Merging .gitignore…");
    match merge_gitignore(target)? {
        GitignoreChange::Created => changes.push("created .gitignore".into()),
        GitignoreChange::ReplacedManaged => {
            changes.push("updated sil-managed .gitignore block".into())
        }
        GitignoreChange::Unchanged => {}
    }

    // Scaffold only when missing (do not clobber user work)
    spinner.set_message("Filling missing scaffold files…");
    let missing = write_scaffold_readmes(target, /*overwrite*/ false)?;
    for m in missing {
        changes.push(format!("created {m}"));
    }

    for (rel_path, content, label) in [
        (rel::CONFIG, templates::CONFIG_YAML, "config.yaml"),
        (rel::STRUCTURE, templates::STRUCTURE_YAML, "structure.yaml"),
        (
            rel::PAPER_DRAFT,
            templates::PAPER_DRAFT_TEX,
            "paper_draft.tex",
        ),
        (rel::PAPER_FINAL, templates::PAPER_FINAL_TEX, "paper.tex"),
        (rel::REFERENCES, templates::REFERENCES_BIB, "references.bib"),
        (rel::README, templates::PROJECT_README, "README.md"),
    ] {
        if write_if_missing(target, rel_path, content)? {
            changes.push(format!("created missing {label}"));
        }
    }

    if paths.paper_draft().is_file() {
        spinner.set_message("Refreshing draft section cache…");
        match write_initial_draft_sections(target) {
            Ok(n) => changes.push(format!("refreshed .sil/draft_sections/ ({n} sections)")),
            Err(e) => ui.warn(&format!("draft section split skipped: {e}")),
        }
    }

    spinner.set_message("Ensuring SQLite database…");
    SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("database: {e}"))?;

    spinner.set_message("Ensuring git repository…");
    init_repo(target).map_err(|e| anyhow::anyhow!("{e}"))?;

    spinner.finish_success(&format!("Project updated at {target}"));

    ui.success("Updated sil workspace to current template version");
    for c in &changes {
        ui.muted(&format!("  • {c}"));
    }
    ui.muted("  preserved: config.yaml, structure.yaml, manuscripts, custom gitignore rules");

    let body = if changes.is_empty() {
        "Project already matched current sil templates.".into()
    } else {
        format!(
            "Applied:\n{}",
            changes
                .iter()
                .map(|c| format!("- {c}"))
                .collect::<Vec<_>>()
                .join("\n")
        )
    };
    let proposal =
        CommitProposal::new("Update sil project templates", SciAction::Update).with_body(body);
    print_proposal(ui, &proposal);

    Ok(())
}

fn print_proposal(ui: &dyn SilUi, proposal: &CommitProposal) {
    ui.println("");
    ui.info("Commit proposal (not applied — never auto-committed):");
    ui.muted("---");
    for line in proposal.message().lines() {
        ui.muted(line);
    }
    ui.muted("---");
    ui.muted("To apply: git add -A && git commit with the message above.");
}

fn ensure_layout(target: &Utf8Path) -> Result<()> {
    let paths = ProjectPaths::new(target);
    let dirs = [
        paths.sil_dir(),
        paths.skills_dir(),
        paths.improvement_dir(),
        paths.draft_sections_dir(),
        paths.join(rel::SOURCES),
        paths.join(rel::DATA),
        paths.join(rel::FIGURES),
        paths.join(rel::FIGURES_PLOTS),
        paths.join(rel::FIGURES_IMAGES),
        paths.join(rel::AGENT),
    ];
    for d in &dirs {
        fs::create_dir_all(d.as_str()).with_context(|| format!("create {d}"))?;
    }
    Ok(())
}

fn write_skills(target: &Utf8Path) -> Result<()> {
    write(target, rel::SKILL_SYSTEM, templates::SKILL_SYSTEM)?;
    write(target, rel::SKILL_PAPER, templates::SKILL_PAPER)?;
    write(target, rel::SKILL_AGENT_CODE, templates::SKILL_AGENT_CODE)?;
    Ok(())
}

/// Split paper_draft.tex into `.sil/draft_sections/`. Returns section count.
fn write_initial_draft_sections(target: &Utf8Path) -> Result<usize> {
    let paths = ProjectPaths::new(target);
    let draft = paths.paper_draft();
    if !draft.is_file() {
        return Ok(0);
    }
    let out = paths.draft_sections_dir();
    let (_src, written) = write_draft_sections_from_file(&draft, &out)
        .map_err(|e| anyhow::anyhow!("draft section split: {e}"))?;
    Ok(written.len())
}

/// Write folder README templates. When `overwrite` is false, only create missing files.
/// Returns relative paths that were created.
fn write_scaffold_readmes(target: &Utf8Path, overwrite: bool) -> Result<Vec<String>> {
    let files = [
        ("sources/README.md", templates::SOURCES_README),
        ("data/README.md", templates::DATA_README),
        ("figures/plots/README.md", templates::FIGURES_PLOTS_README),
        ("figures/images/README.md", templates::FIGURES_IMAGES_README),
        ("agent/README.md", templates::AGENT_README),
        (".sil/improvement/README.md", templates::IMPROVEMENT_README),
    ];
    let mut created = Vec::new();
    for (rel_path, content) in files {
        if overwrite {
            write(target, rel_path, content)?;
        } else if write_if_missing(target, rel_path, content)? {
            created.push(rel_path.to_string());
        }
    }
    Ok(created)
}

fn write(root: &Utf8Path, rel_path: &str, content: &str) -> Result<()> {
    let path = root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent.as_str())?;
    }
    fs::write(path.as_str(), content).with_context(|| format!("write {path}"))?;
    Ok(())
}

/// Write only when the file does not exist. Returns true if created.
fn write_if_missing(root: &Utf8Path, rel_path: &str, content: &str) -> Result<bool> {
    let path = root.join(rel_path);
    if path.is_file() {
        return Ok(false);
    }
    write(root, rel_path, content)?;
    Ok(true)
}

#[derive(Debug, PartialEq, Eq)]
enum GitignoreChange {
    Created,
    ReplacedManaged,
    Unchanged,
}

/// Merge the sil-managed `.gitignore` block into the project file.
fn merge_gitignore(root: &Utf8Path) -> Result<GitignoreChange> {
    let path = root.join(".gitignore");
    let managed = managed_gitignore_block();

    if !path.is_file() {
        write(root, ".gitignore", templates::GITIGNORE)?;
        return Ok(GitignoreChange::Created);
    }

    let existing = fs::read_to_string(path.as_str()).with_context(|| format!("read {path}"))?;
    let new_content = merge_gitignore_text(&existing, &managed);
    if new_content == existing {
        return Ok(GitignoreChange::Unchanged);
    }
    fs::write(path.as_str(), &new_content).with_context(|| format!("write {path}"))?;
    Ok(GitignoreChange::ReplacedManaged)
}

/// Extract the managed block (including markers) from the default template.
fn managed_gitignore_block() -> String {
    let start = templates::GITIGNORE_MANAGED_START;
    let end = templates::GITIGNORE_MANAGED_END;
    let Some(start_idx) = templates::GITIGNORE.find(start) else {
        return templates::GITIGNORE.to_string();
    };
    let rest = &templates::GITIGNORE[start_idx..];
    let Some(end_rel) = rest.find(end) else {
        return templates::GITIGNORE.to_string();
    };
    let end_idx = end_rel + end.len();
    rest[..end_idx].to_string()
}

/// Rebuild `.gitignore` text: refresh managed block, keep custom rules.
fn merge_gitignore_text(existing: &str, managed_block: &str) -> String {
    let start = templates::GITIGNORE_MANAGED_START;
    let end = templates::GITIGNORE_MANAGED_END;

    if let (Some(s), Some(e_rel)) = (
        existing.find(start),
        existing
            .find(start)
            .and_then(|s| existing[s..].find(end).map(|r| s + r)),
    ) {
        let e = e_rel + end.len();
        let before = existing[..s].trim_end();
        let after = existing[e..].trim_start_matches(['\r', '\n']);
        let mut out = String::new();
        if !before.is_empty() {
            out.push_str(before);
            out.push_str("\n\n");
        }
        out.push_str(managed_block);
        out.push('\n');
        if !after.is_empty() {
            // Preserve a blank line before custom rules when useful
            if !after.starts_with('#') && !after.starts_with('\n') {
                out.push('\n');
            }
            out.push_str(after);
            if !after.ends_with('\n') {
                out.push('\n');
            }
        } else {
            out.push('\n');
            out.push_str("# Custom rules below this line are preserved by `sil init --update`.\n");
        }
        return out;
    }

    // Legacy / hand-written gitignore without markers: keep user content, prepend managed block.
    let trimmed = existing.trim();
    if trimmed.is_empty() {
        return templates::GITIGNORE.to_string();
    }
    // Already identical to full template
    if existing == templates::GITIGNORE {
        return existing.to_string();
    }
    format!(
        "{managed}\n\n# --- preserved previous .gitignore (no sil-managed markers) ---\n{existing}",
        managed = managed_block,
        existing = if existing.ends_with('\n') {
            existing.to_string()
        } else {
            format!("{existing}\n")
        }
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_refreshes_managed_block_preserves_custom() {
        let old = format!(
            "{start}\nold content\n{end}\n\n# my custom\n*.secret\n",
            start = templates::GITIGNORE_MANAGED_START,
            end = templates::GITIGNORE_MANAGED_END,
        );
        let managed = managed_gitignore_block();
        let merged = merge_gitignore_text(&old, &managed);
        assert!(merged.contains(templates::GITIGNORE_MANAGED_START));
        assert!(merged.contains(".sil/db.sqlite"));
        assert!(merged.contains("# my custom"));
        assert!(merged.contains("*.secret"));
        assert!(!merged.contains("old content"));
    }

    #[test]
    fn merge_legacy_prepends_managed() {
        let legacy = "# my rules\n*.tmp\n";
        let managed = managed_gitignore_block();
        let merged = merge_gitignore_text(legacy, &managed);
        assert!(merged.starts_with(templates::GITIGNORE_MANAGED_START));
        assert!(merged.contains("preserved previous"));
        assert!(merged.contains("*.tmp"));
    }

    #[test]
    fn managed_block_has_markers_and_db() {
        let block = managed_gitignore_block();
        assert!(block.starts_with(templates::GITIGNORE_MANAGED_START));
        assert!(
            block.ends_with(templates::GITIGNORE_MANAGED_END)
                || block.contains(templates::GITIGNORE_MANAGED_END)
        );
        assert!(block.contains(".sil/db.sqlite"));
        assert!(block.contains("figures/images/**"));
        assert!(block.contains("data/**"));
        // Must not ignore improvement proposals or draft section cache
        assert!(!block.lines().any(|l| {
            let t = l.trim();
            t == ".sil/"
                || t == ".sil/**"
                || t == ".sil/improvement"
                || t == ".sil/improvement/"
                || t == ".sil/draft_sections"
                || t == ".sil/draft_sections/"
        }));
        assert!(block.contains("improvement/") || block.contains("draft_sections/"));
    }
}
