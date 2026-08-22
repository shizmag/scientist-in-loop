//! `sil context`

use std::fs;

use anyhow::Result;
use sil_agent::{
    ContextFlags, ContextInput, SkillSelection, generate_context, generate_context_envelope,
    generate_context_json, sources_summary,
};
use sil_core::{SilUi, Structure};
use sil_db::SilDb;
use sil_git::log_entries;

use crate::util::load_project;

pub fn run(
    flags: ContextFlags,
    task: Option<&str>,
    json: bool,
    compact: bool,
    envelope: bool,
    ui: &dyn SilUi,
) -> Result<()> {
    let (root, _config, paths) = load_project()?;
    let config_yaml = fs::read_to_string(paths.config().as_str()).unwrap_or_default();
    let structure_yaml = fs::read_to_string(paths.structure().as_str()).unwrap_or_default();
    let structure = Structure::load(&paths.structure()).ok();
    let summary = if paths.db().is_file() {
        if let Ok(db) = SilDb::open(&paths.db()) {
            sources_summary(&db).unwrap_or_default()
        } else {
            String::new()
        }
    } else {
        String::new()
    };
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

    if json {
        if envelope {
            let env = generate_context_envelope(&input).map_err(|e| anyhow::anyhow!("{e}"))?;
            if compact {
                println!("{}", serde_json::to_string(&env)?);
            } else {
                println!("{}", serde_json::to_string_pretty(&env)?);
            }
        } else {
            let json_str =
                generate_context_json(&input, compact).map_err(|e| anyhow::anyhow!("{e}"))?;
            println!("{json_str}");
        }
    } else {
        let ctx = generate_context(&input).map_err(|e| anyhow::anyhow!("{e}"))?;
        // Context is primary payload — print plain for piping.
        println!("{ctx}");
    }
    let _ = ui;
    Ok(())
}
