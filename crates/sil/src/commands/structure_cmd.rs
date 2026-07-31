//! `sil structure` — update structure.yaml with Sci-Action proposals.

use anyhow::{Result, bail};
use sil_core::{SciAction, SectionCompletion, SilUi, Structure};
use sil_git::CommitProposal;

use crate::util::{load_project, print_proposal};

/// Set a section's completion and propose an update-structure commit.
pub fn set_completion(section_id: &str, completion: &str, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let completion: SectionCompletion = completion
        .parse()
        .map_err(|e| anyhow::anyhow!("invalid completion: {e}"))?;

    let mut structure = Structure::load(&paths.structure()).map_err(|e| anyhow::anyhow!("{e}"))?;
    let prev = structure
        .set_section_completion(section_id, completion)
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    structure
        .save(&paths.structure())
        .map_err(|e| anyhow::anyhow!("{e}"))?;

    ui.success(&format!("Section '{section_id}': {prev} → {completion}"));
    ui.muted(&format!("  {}", structure.completion_summary()));

    let proposal = CommitProposal::new(
        format!("Update structure: {section_id} → {completion}"),
        SciAction::UpdateStructure,
    )
    .with_body(format!(
        "Set section `{section_id}` completion from `{prev}` to `{completion}`.\n\
         Summary: {}.",
        structure.completion_summary()
    ));
    print_proposal(ui, &proposal);
    Ok(())
}

/// List sections and completions (human-readable).
pub fn list(ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let structure = Structure::load(&paths.structure()).map_err(|e| anyhow::anyhow!("{e}"))?;
    if structure.sections.is_empty() {
        bail!("no sections in structure.yaml");
    }
    ui.info(&format!("Structure: {}", structure.completion_summary()));
    for sec in &structure.sections {
        ui.println(&format!(
            "  [{}] {} — {}",
            sec.completion, sec.id, sec.title
        ));
    }
    Ok(())
}
