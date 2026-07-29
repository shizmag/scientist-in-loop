//! `sil propose` — print a Sci-Action commit proposal (never auto-commits).

use anyhow::{Result, bail};
use sil_core::{SciAction, SilUi};
use sil_git::{propose_from_status, proposal_for_action, status as git_status};

use crate::util::{load_project, print_proposal};

/// Produce a commit proposal from dirty paths and/or an explicit Sci-Action.
pub fn run(
    action: Option<&str>,
    subject: Option<&str>,
    body: Option<&str>,
    ui: &dyn SilUi,
) -> Result<()> {
    let (root, _config, _paths) = load_project()?;
    let explicit = match action {
        Some(a) => Some(
            a.parse::<SciAction>()
                .map_err(|e| anyhow::anyhow!("invalid --action: {e}"))?,
        ),
        None => None,
    };

    let st = git_status(&root).map_err(|e| anyhow::anyhow!("{e}"))?;
    if !st.is_repo {
        bail!("not a git repository; run `sil init` or `git init` first");
    }

    let proposal = if let Some(a) = explicit {
        // Explicit action always wins; still attach dirty-path body when useful.
        match propose_from_status(&st, Some(a), subject, body) {
            Ok(p) => p,
            Err(_) => proposal_for_action(a, subject, body),
        }
    } else {
        propose_from_status(&st, None, subject, body).map_err(|e| anyhow::anyhow!("{e}"))?
    };

    ui.info("Commit proposal only — nothing was committed.");
    print_proposal(ui, &proposal);
    Ok(())
}
