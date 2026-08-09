//! `sil paper estimate` — L0 manuscript quality estimate (read-only).

use anyhow::Result;
use sil_agent::{
    EstimateInput, EstimateMode, estimate_proposal_message, report_to_markdown,
    run_heuristic_estimate, write_estimate_report,
};
use sil_core::SilUi;

use crate::util::load_project;

/// Run heuristic estimate and optionally write under `.sil/reviews/`.
pub fn run(mode: &str, json: bool, write: bool, ui: &dyn SilUi) -> Result<()> {
    let (root, _config, paths) = load_project()?;
    let structure = sil_core::Structure::load(&paths.structure()).ok();
    let mode = EstimateMode::parse(mode);

    let report = run_heuristic_estimate(&EstimateInput {
        root: &root,
        mode,
        structure: structure.as_ref(),
    })?;

    if write {
        let dir = write_estimate_report(&root, &report)?;
        if json {
            let mut value = serde_json::to_value(&report)?;
            if let Some(obj) = value.as_object_mut() {
                obj.insert("report_dir".into(), serde_json::json!(dir.to_string()));
                obj.insert(
                    "proposal".into(),
                    serde_json::json!(estimate_proposal_message(&dir)),
                );
                obj.insert("never_committed".into(), serde_json::json!(true));
            }
            println!("{}", serde_json::to_string_pretty(&value)?);
        } else {
            ui.success(&format!("Wrote estimate report to {dir}"));
            ui.println(&report_to_markdown(&report));
            ui.println("");
            ui.info("Commit proposal (not applied — never auto-committed):");
            ui.muted("---");
            for line in estimate_proposal_message(&dir).lines() {
                ui.muted(line);
            }
            ui.muted("---");
        }
    } else if json {
        println!("{}", serde_json::to_string_pretty(&report)?);
    } else {
        ui.println(&report_to_markdown(&report));
        ui.muted("Tip: re-run with --write to save under .sil/reviews/");
    }

    Ok(())
}
