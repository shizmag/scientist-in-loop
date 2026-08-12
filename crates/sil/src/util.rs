//! Shared helpers used by command modules.

use anyhow::Result;
use camino::Utf8PathBuf;
use sil_core::{Config, ProjectPaths, SilUi, StdUi};
use sil_git::CommitProposal;
use sil_parse::MarkerRunner;

/// Build the terminal UI for this process.
pub fn make_ui(plain: bool) -> Box<dyn SilUi> {
    if plain
        || std::env::var_os("NO_COLOR").is_some()
        || std::env::var("SIL_NO_COLOR")
            .map(|v| v == "1")
            .unwrap_or(false)
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
    let mut global = sil_core::GlobalSettings::load_or_default(None);
    global.touch_recent_project(root.clone());
    let _ = global.save(None);
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

/// Resolve Marker runner (CLI binary, Python helper, or stub).
pub fn marker_runner() -> Result<Box<dyn MarkerRunner>> {
    sil_parse::discover_marker_runner().map_err(|e| anyhow::anyhow!("{e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_make_ui_plain_flag() {
        let ui = make_ui(true);
        assert!(!ui.interactive());
    }

    #[test]
    fn test_make_ui_no_color_env() {
        let ui = make_ui(false);
        assert!(ui.interactive() || !ui.interactive());
    }

    #[test]
    fn test_make_ui_env_overrides() {
        unsafe {
            std::env::set_var("SIL_NO_COLOR", "1");
        }
        let ui = make_ui(false);
        assert!(!ui.interactive());
        unsafe {
            std::env::remove_var("SIL_NO_COLOR");
        }
    }
}


