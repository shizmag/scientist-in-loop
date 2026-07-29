//! E2E: `sil build` (invokes configured engine or clear error).

mod common;

use common::{init_project, sil};

#[test]
fn build_invokes_engine_or_errors_clearly() {
    let (_dir, project) = init_project("buildp");

    // Default engine is tectonic. On machines with tectonic this succeeds;
    // otherwise we still require a clean, actionable failure.
    let assert = sil().current_dir(&project).arg("build").assert();
    let output = assert.get_output();
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    let combined = format!("{stdout}{stderr}");

    if output.status.success() {
        assert!(
            combined.contains("PDF:") || combined.contains("Built"),
            "success path should mention PDF:\n{combined}"
        );
    } else {
        assert!(
            combined.contains("not found")
                || combined.contains("build failed")
                || combined.contains("LaTeX")
                || combined.contains("tectonic"),
            "failure must be human-readable:\n{combined}"
        );
    }
}
