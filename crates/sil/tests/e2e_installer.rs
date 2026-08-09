//! End-to-end integration test for `sil mcp install`.

use std::fs;
use tempfile::TempDir;
use serde_json::Value;

#[test]
fn test_mcp_installer_gemini() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();

    std::env::set_var("HOME", home);

    let output = assert_cmd::Command::cargo_bin("sil")
        .unwrap()
        .args(&["mcp", "install", "--client", "gemini"])
        .output()
        .expect("failed to execute sil mcp install");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let config_path = home.join(".gemini/antigravity-cli/mcp/scientist-in-loop.json");
    assert!(config_path.exists(), "Expected config file at {}", config_path.display());

    let content = fs::read_to_string(&config_path).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();

    assert!(json.get("command").is_some());
    assert_eq!(json["args"], serde_json::json!(["project", "mcp", "--quiet"]));
}

#[test]
fn test_mcp_installer_claude_merged() {
    let temp = TempDir::new().unwrap();
    let home = temp.path();

    std::env::set_var("HOME", home);

    let claude_dir = home.join("Library/Application Support/Claude");
    fs::create_dir_all(&claude_dir).unwrap();
    let config_path = claude_dir.join("claude_desktop_config.json");
    fs::write(&config_path, r#"{"mcpServers":{"other-server":{"command":"other"}}}"#).unwrap();

    let output = assert_cmd::Command::cargo_bin("sil")
        .unwrap()
        .args(&["mcp", "install", "--client", "claude"])
        .output()
        .expect("failed to execute sil mcp install");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));

    let content = fs::read_to_string(&config_path).unwrap();
    let json: Value = serde_json::from_str(&content).unwrap();

    assert!(json["mcpServers"]["other-server"]["command"] == "other");
    assert!(json["mcpServers"]["scientist-in-loop"]["command"].is_string());
    assert_eq!(json["mcpServers"]["scientist-in-loop"]["args"], serde_json::json!(["project", "mcp", "--quiet"]));
}

#[test]
fn test_mcp_installer_custom_path() {
    let temp = TempDir::new().unwrap();
    let custom_file = temp.path().join("custom_mcp.json");

    let output = assert_cmd::Command::cargo_bin("sil")
        .unwrap()
        .args(&["mcp", "install", "--client", "custom", "--path", custom_file.to_str().unwrap()])
        .output()
        .expect("failed to execute sil mcp install");

    assert!(output.status.success(), "stderr: {}", String::from_utf8_lossy(&output.stderr));
    assert!(custom_file.exists());
}
