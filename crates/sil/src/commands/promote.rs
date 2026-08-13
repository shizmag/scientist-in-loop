//! `sil promote` — copy paper_draft.tex → paper.tex and propose Sci-Action.

use std::fs;

use anyhow::{Context, Result, bail};
use sil_core::{SciAction, SectionCompletion, SilUi, Structure, paths::rel};
use sil_git::CommitProposal;

use crate::util::{load_project, print_proposal};

/// Promote draft manuscript to final shell, with optional structure guardrails.
pub fn run(force: bool, ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let draft = paths.paper_draft();
    let final_tex = paths.paper_final();

    if !draft.is_file() {
        bail!("missing {}; nothing to promote", draft);
    }

    let structure = Structure::load(&paths.structure()).map_err(|e| anyhow::anyhow!("{e}"))?;
    if !force {
        let ready: Vec<_> = structure
            .sections
            .iter()
            .filter(|s| {
                matches!(
                    s.completion,
                    SectionCompletion::Draft | SectionCompletion::Polished
                )
            })
            .collect();
        if ready.is_empty() && !structure.sections.is_empty() {
            bail!(
                "no sections are at least `draft` in {}; pass --force to promote anyway",
                rel::STRUCTURE
            );
        }
        if !structure.sections.is_empty() {
            ui.muted(&format!(
                "Guardrail: {}/{} section(s) at draft/polished",
                ready.len(),
                structure.sections.len()
            ));
        }
    } else {
        ui.warn("Promote forced; structure completion guardrail skipped.");
    }

    let content = fs::read_to_string(draft.as_str()).with_context(|| format!("read {draft}"))?;
    sil_core::write_atomic_str(&final_tex, &content)
        .with_context(|| format!("write {final_tex}"))?;
    ui.success(&format!(
        "Copied {} → {} ({} bytes)",
        rel::PAPER_DRAFT,
        rel::PAPER_FINAL,
        content.len()
    ));

    let proposal = CommitProposal::new(
        "Promote paper_draft.tex to paper.tex",
        SciAction::PromoteToFinal,
    )
    .with_body(format!(
        "Copied working draft to final manuscript path.\n\
         Structure summary: {}.",
        structure.completion_summary()
    ));
    print_proposal(ui, &proposal);
    Ok(())
}
