//! E2E: `sil settings` / `sil tui` CLI commands.

mod common;

use common::sil;

#[test]
fn help_lists_settings_and_tui_alias() {
    let assert = sil().arg("--help").assert();
    let output = String::from_utf8_lossy(&assert.get_output().stdout);

    assert!(output.contains("settings"), "help output should list settings subcommand");
}
