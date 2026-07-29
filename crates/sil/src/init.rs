//! `sil init` — create a full project workspace from templates.

use std::fs;

use anyhow::{Context, Result, bail};
use camino::Utf8Path;

use sil_core::{ProjectPaths, SciAction, SilUi, paths::rel};
use sil_db::SilDb;
use sil_git::{CommitProposal, init_repo};

use crate::templates;

/// Create a new sil project at `target`.
pub fn init_project(target: &Utf8Path, ui: &dyn SilUi) -> Result<()> {
    if target.join(rel::CONFIG).is_file() {
        bail!(
            "already a sil project: {} exists",
            target.join(rel::CONFIG)
        );
    }

    let mut spinner = ui.spinner(&format!("Initialising project at {target}"));

    fs::create_dir_all(target.as_str())
        .with_context(|| format!("create project root {target}"))?;

    let paths = ProjectPaths::new(target);

    // Directories
    let dirs = [
        paths.sil_dir(),
        paths.skills_dir(),
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

    spinner.set_message("Writing templates…");

    // Core config / structure
    write(target, rel::CONFIG, templates::CONFIG_YAML)?;
    write(target, rel::STRUCTURE, templates::STRUCTURE_YAML)?;
    write(
        target,
        ".sil/structure.example.yaml",
        templates::STRUCTURE_EXAMPLE_YAML,
    )?;

    // Skills
    write(target, rel::SKILL_SYSTEM, templates::SKILL_SYSTEM)?;
    write(target, rel::SKILL_PAPER, templates::SKILL_PAPER)?;
    write(target, rel::SKILL_AGENT_CODE, templates::SKILL_AGENT_CODE)?;

    // Paper stubs
    write(target, rel::PAPER_DRAFT, templates::PAPER_DRAFT_TEX)?;
    write(target, rel::PAPER_FINAL, templates::PAPER_FINAL_TEX)?;
    write(target, rel::REFERENCES, templates::REFERENCES_BIB)?;

    // Folder READMEs
    write(target, "data/README.md", templates::DATA_README)?;
    write(
        target,
        "figures/plots/README.md",
        templates::FIGURES_PLOTS_README,
    )?;
    write(
        target,
        "figures/images/README.md",
        templates::FIGURES_IMAGES_README,
    )?;
    write(target, "agent/README.md", templates::AGENT_README)?;
    write(target, rel::README, templates::PROJECT_README)?;

    // .gitignore
    write(target, ".gitignore", templates::GITIGNORE)?;

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

    let proposal = CommitProposal::new("Initialize sil project", SciAction::Init).with_body(
        "Created directory layout, templates, SQLite+FTS5 database, and skills.",
    );
    ui.println("");
    ui.info("Commit proposal (not applied — never auto-committed):");
    ui.muted("---");
    for line in proposal.message().lines() {
        ui.muted(line);
    }
    ui.muted("---");
    ui.muted("To apply: git add -A && git commit with the message above.");

    Ok(())
}

fn write(root: &Utf8Path, rel_path: &str, content: &str) -> Result<()> {
    let path = root.join(rel_path);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent.as_str())?;
    }
    fs::write(path.as_str(), content).with_context(|| format!("write {path}"))?;
    Ok(())
}
