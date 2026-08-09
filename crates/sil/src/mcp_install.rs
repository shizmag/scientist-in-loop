//! MCP Client Auto-Installer (`sil mcp install`).

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use dialoguer::{Input, Select};
use serde_json::{json, Value};

/// Supported target AI clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetClient {
    GeminiAntigravity,
    Grok,
    ClaudeDesktop,
    Cursor,
    Custom,
}

const MENU_OPTIONS: &[&str] = &[
    "Gemini / Antigravity",
    "Grok",
    "Claude Desktop",
    "Cursor",
    "Custom Path",
];

impl TargetClient {
    pub fn parse_str(s: &str) -> Result<Self> {
        let normalized = s.trim().to_lowercase();
        match normalized.as_str() {
            "gemini" | "antigravity" | "gemini / antigravity" | "gemini-antigravity" => {
                Ok(TargetClient::GeminiAntigravity)
            }
            "grok" => Ok(TargetClient::Grok),
            "claude" | "claude-desktop" | "claude desktop" => Ok(TargetClient::ClaudeDesktop),
            "cursor" => Ok(TargetClient::Cursor),
            "custom" => Ok(TargetClient::Custom),
            _ => bail!(
                "Unknown client '{s}'. Supported clients: gemini, grok, claude, cursor, custom"
            ),
        }
    }
}

/// Determine user's home directory, honoring `HOME` env var override.
pub fn get_home_dir() -> Option<PathBuf> {
    if let Some(home) = std::env::var_os("HOME")
        && !home.is_empty()
    {
        return Some(PathBuf::from(home));
    }
    dirs::home_dir()
}

/// Determine the target configuration file path for a given client.
pub fn get_config_path(client: TargetClient, custom_path: Option<&Path>) -> Result<PathBuf> {
    let home = get_home_dir().context("Could not determine user HOME directory")?;

    match client {
        TargetClient::GeminiAntigravity => {
            Ok(home.join(".gemini/antigravity-cli/mcp/scientist-in-loop.json"))
        }
        TargetClient::Grok => Ok(home.join(".grok/plugins/scientist-in-loop/.mcp.json")),
        TargetClient::ClaudeDesktop => {
            Ok(home.join("Library/Application Support/Claude/claude_desktop_config.json"))
        }
        TargetClient::Cursor => Ok(home.join(".cursor/mcp.json")),
        TargetClient::Custom => {
            if let Some(path) = custom_path {
                Ok(path.to_path_buf())
            } else {
                bail!("Custom path must be provided via --path when using --client custom non-interactively")
            }
        }
    }
}

/// Get the path to the currently running `sil` binary or fallback to `"sil"`.
pub fn get_sil_binary_path() -> String {
    if let Ok(exe) = std::env::current_exe() {
        if let Ok(canonical) = exe.canonicalize() {
            return canonical.to_string_lossy().to_string();
        }
        return exe.to_string_lossy().to_string();
    }
    "sil".to_string()
}

/// Options for `sil mcp install`.
#[derive(Debug, Default, Clone)]
pub struct InstallOptions {
    pub client: Option<String>,
    pub path: Option<PathBuf>,
}

/// Run the MCP client auto-installer.
pub fn run_installer(options: InstallOptions) -> Result<PathBuf> {
    let (target_client, custom_path) = match options.client {
        Some(client_str) => {
            let client = TargetClient::parse_str(&client_str)?;
            let mut custom_p = options.path;
            if client == TargetClient::Custom && custom_p.is_none() {
                let path_str: String = Input::new()
                    .with_prompt("Enter configuration file path")
                    .interact()
                    .context("Failed to read custom path in interactive mode")?;
                custom_p = Some(PathBuf::from(path_str));
            }
            (client, custom_p)
        }
        None => {
            let selection = Select::new()
                .with_prompt("Select target AI client for sil MCP installation")
                .items(MENU_OPTIONS)
                .default(0)
                .interact()
                .context("Interactive selection failed (specify --client in non-interactive environments)")?;

            let client = match selection {
                0 => TargetClient::GeminiAntigravity,
                1 => TargetClient::Grok,
                2 => TargetClient::ClaudeDesktop,
                3 => TargetClient::Cursor,
                4 => TargetClient::Custom,
                _ => unreachable!(),
            };

            let custom_p = if client == TargetClient::Custom {
                if let Some(p) = options.path {
                    Some(p)
                } else {
                    let path_str: String = Input::new()
                        .with_prompt("Enter configuration file path")
                        .interact()
                        .context("Failed to read custom path in interactive mode")?;
                    Some(PathBuf::from(path_str))
                }
            } else {
                options.path
            };

            (client, custom_p)
        }
    };

    let config_path = get_config_path(target_client, custom_path.as_deref())?;

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create parent directory {}", parent.display()))?;
    }

    let binary_path = get_sil_binary_path();
    let mcp_entry = json!({
        "command": binary_path,
        "args": ["project", "mcp", "--quiet"]
    });

    let is_merged_config = matches!(target_client, TargetClient::ClaudeDesktop | TargetClient::Cursor);

    let content_to_write = if is_merged_config {
        let mut root = if config_path.exists() {
            let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
            serde_json::from_str::<Value>(&existing).unwrap_or_else(|_| json!({}))
        } else {
            json!({})
        };

        if !root.is_object() {
            root = json!({});
        }

        if !root.get("mcpServers").is_some_and(|v| v.is_object()) {
            root["mcpServers"] = json!({});
        }

        root["mcpServers"]["scientist-in-loop"] = mcp_entry;
        serde_json::to_string_pretty(&root)?
    } else {
        if target_client == TargetClient::Custom && config_path.exists() {
            let existing = std::fs::read_to_string(&config_path).unwrap_or_default();
            if let Ok(mut root) = serde_json::from_str::<Value>(&existing) {
                if root.is_object() && root.get("mcpServers").is_some_and(|v| v.is_object()) {
                    root["mcpServers"]["scientist-in-loop"] = mcp_entry;
                    serde_json::to_string_pretty(&root)?
                } else {
                    serde_json::to_string_pretty(&mcp_entry)?
                }
            } else {
                serde_json::to_string_pretty(&mcp_entry)?
            }
        } else {
            serde_json::to_string_pretty(&mcp_entry)?
        }
    };

    std::fs::write(&config_path, format!("{content_to_write}\n"))
        .with_context(|| format!("Failed to write MCP config to {}", config_path.display()))?;

    println!("✔ Installed sil MCP server config to {}", config_path.display());

    Ok(config_path)
}
