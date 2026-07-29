//! `sil context`

use std::fs;

use anyhow::Result;
use sil_agent::{ContextFlags, ContextInput, SkillSelection, generate_context, sources_summary};
use sil_core::{SilUi, Structure};
use sil_db::SilDb;
use sil_git::log_entries;

use crate::util::load_project;

pub fn run(flags: ContextFlags, task: Option<&str>, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, paths) = load_project()?;
    let config_yaml = fs::read_to_string(paths.config().as_str())?;
    let structure_yaml = fs::read_to_string(paths.structure().as_str())?;
    let structure = Structure::load(&paths.structure()).ok();
    let db = SilDb::open(&paths.db()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let summary = sources_summary(&db).map_err(|e| anyhow::anyhow!("{e}"))?;
    let log = log_entries(&root, 15, true).unwrap_or_default();

    let mut skills = if let Some(t) = task {
        SkillSelection::from_task(t)
    } else {
        SkillSelection::always()
    };
    skills.merge_flags(&flags);

    let input = ContextInput {
        root: &root,
        config_yaml: &config_yaml,
        structure_yaml: &structure_yaml,
        structure: structure.as_ref(),
        sources_summary: &summary,
        log_entries: &log,
        flags: &flags,
        skills,
    };
    let ctx = generate_context(&input).map_err(|e| anyhow::anyhow!("{e}"))?;
    // Context is primary payload — print plain for piping.
    println!("{ctx}");
    let _ = ui;
    Ok(())
}
