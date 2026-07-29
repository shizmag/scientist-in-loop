//! `sil status`

use anyhow::Result;
use sil_core::{SilUi, Structure, paths::rel};
use sil_db::SilDb;
use sil_git::{path_has_changes, status as git_status};

use crate::util::load_project;

pub fn run(ui: &dyn SilUi) -> Result<()> {
    let (root, config, paths) = load_project()?;
    let structure = Structure::load(&paths.structure()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let source_count = db.source_count().map_err(|e| anyhow::anyhow!("{e}"))?;
    let parsed_count = db.parsed_count().map_err(|e| anyhow::anyhow!("{e}"))?;
    let git = git_status(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    let draft_dirty = path_has_changes(&root, rel::PAPER_DRAFT).unwrap_or(false);
    let summary = structure.completion_summary();

    ui.println("");
    ui.info(&format!("Project: {root}"));
    ui.println(&format!("  title:  {}", config.project.title));
    ui.println(&format!("  stage:  {}", config.project.stage));
    ui.println(&format!(
        "  latex:  {} → {}",
        config.latex.engine, config.latex.main
    ));
    ui.println("");
    ui.info("Sources");
    ui.println(&format!(
        "  database: {source_count} source(s), {parsed_count} parsed"
    ));
    ui.println("");
    ui.info("Structure");
    ui.println(&format!("  {summary}"));
    for sec in &structure.sections {
        ui.muted(&format!(
            "  - [{}] {} ({})",
            sec.completion, sec.id, sec.title
        ));
    }
    ui.println("");
    ui.info("Git");
    if !git.is_repo {
        ui.warn("  not a git repository");
    } else if git.clean {
        ui.success("  working tree clean");
    } else {
        ui.warn(&format!("  {} uncommitted change(s)", git.entries.len()));
        for e in git.entries.iter().take(12) {
            ui.muted(&format!("    {e}"));
        }
        if git.entries.len() > 12 {
            ui.muted(&format!("    … {} more", git.entries.len() - 12));
        }
    }
    if draft_dirty {
        ui.warn("  paper_draft.tex has uncommitted changes");
    } else {
        ui.muted("  paper_draft.tex: no uncommitted changes");
    }
    ui.println("");
    Ok(())
}
