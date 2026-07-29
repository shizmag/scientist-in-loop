//! Shared helpers used by command modules.

use anyhow::Result;
use camino::Utf8PathBuf;
use sil_core::{Config, ProjectPaths, SilUi, StdUi};
use sil_git::CommitProposal;
use sil_parse::{MarkerRunner, PythonMarkerRunner, StubMarkerRunner};

/// Build the terminal UI for this process.
pub fn make_ui(plain: bool) -> Box<dyn SilUi> {
    if plain
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var("SIL_NO_COLOR").map(|v| v == "1").unwrap_or(false)
        || std::env::var("SIL_NONINTERACTIVE")
            .map(|v| v == "1")
            .unwrap_or(false)
    {
        Box::new(StdUi::plain())
    } else {
        Box::new(StdUi::new())
    }
}

/// Load the current project root, config, and path helpers.
pub fn load_project() -> Result<(Utf8PathBuf, Config, ProjectPaths)> {
    let root = sil_core::project_root_from_cwd().map_err(|e| anyhow::anyhow!("{e}"))?;
    let paths = ProjectPaths::new(&root);
    let config = Config::load(&paths.config()).map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok((root, config, paths))
}

/// Print a commit proposal (never applied automatically).
pub fn print_proposal(ui: &dyn SilUi, proposal: &CommitProposal) {
    ui.println("");
    ui.info("Commit proposal (not applied — never auto-committed):");
    ui.muted("---");
    for line in proposal.message().lines() {
        ui.muted(line);
    }
    ui.muted("---");
    ui.muted("To apply: git add -A && git commit with the message above.");
}

/// Resolve Marker runner (stub when `SIL_MARKER_STUB` is set).
pub fn marker_runner() -> Result<Box<dyn MarkerRunner>> {
    if let Ok(stub) = std::env::var("SIL_MARKER_STUB") {
        return Ok(Box::new(StubMarkerRunner { content: stub }));
    }
    let runner = PythonMarkerRunner::discover().map_err(|e| anyhow::anyhow!("{e}"))?;
    Ok(Box::new(runner))
}
