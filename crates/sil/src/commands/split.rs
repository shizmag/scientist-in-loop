//! `sil split` — write agent-readable draft section files under `.sil/draft_sections/`.

use anyhow::{Context, Result, bail};
use sil_core::SilUi;
use sil_latex::write_draft_sections_from_file;

use crate::util::load_project;

/// Split `paper_draft.tex` into `.sil/draft_sections/` without modifying the draft.
pub fn run(ui: &dyn SilUi) -> Result<()> {
    let (_root, _config, paths) = load_project()?;
    let draft = paths.paper_draft();
    if !draft.is_file() {
        bail!(
            "missing {}; write the draft first, then re-run `sil split`",
            draft
        );
    }
    let out = paths.draft_sections_dir();

    let before =
        std::fs::read_to_string(draft.as_str()).with_context(|| format!("read {draft}"))?;

    let mut spinner = ui.spinner("Splitting paper_draft.tex into section files…");
    let (read_back, written) =
        write_draft_sections_from_file(&draft, &out).map_err(|e| anyhow::anyhow!("{e}"))?;

    let after =
        std::fs::read_to_string(draft.as_str()).with_context(|| format!("re-read {draft}"))?;
    if before != after || before != read_back {
        bail!("internal error: paper_draft.tex was modified during split");
    }

    spinner.finish_success(&format!(
        "Wrote {} section file(s) under {}",
        written.len(),
        out
    ));
    for w in &written {
        ui.muted(&format!("  • {} ({})", w.filename, w.title));
    }
    ui.muted(&format!("  index: {}/index.md", out));
    ui.muted("Source of truth remains paper_draft.tex (unchanged).");
    Ok(())
}
