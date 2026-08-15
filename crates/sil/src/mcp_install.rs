//! Safe MCP client configuration adapters.

use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use camino::Utf8PathBuf;
use dialoguer::{Input, Select};
use serde_json::{Map, Value, json};

const SERVER_NAME: &str = "scientist-in-loop";
const OWNER_KEY: &str = "x-sil";
const MENU_OPTIONS: &[&str] = &[
    "Gemini / Antigravity",
    "Grok",
    "Claude Desktop",
    "Cursor",
    "OpenCode",
    "Custom Path",
];

/// Supported target AI clients.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetClient {
    GeminiAntigravity,
    Grok,
    ClaudeDesktop,
    Cursor,
    OpenCode,
    Custom,
}

/// Host platforms used by the path adapters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostPlatform {
    Macos,
    Linux,
    Windows,
}

/// Result of an installer status query.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstallStatus {
    /// Resolved host configuration path.
    pub path: PathBuf,
    /// Whether the owned server entry is present.
    pub installed: bool,
}

impl TargetClient {
    pub fn parse_str(s: &str) -> Result<Self> {
        match s.trim().to_lowercase().as_str() {
            "gemini" | "antigravity" | "gemini / antigravity" | "gemini-antigravity" => {
                Ok(Self::GeminiAntigravity)
            }
            "grok" => Ok(Self::Grok),
            "claude" | "claude-desktop" | "claude desktop" => Ok(Self::ClaudeDesktop),
            "cursor" => Ok(Self::Cursor),
            "opencode" | "open-code" => Ok(Self::OpenCode),
            "custom" => Ok(Self::Custom),
            _ => bail!(
                "Unknown client '{s}'. Supported clients: gemini, grok, claude, cursor, opencode, custom"
            ),
        }
    }
}

fn current_platform() -> HostPlatform {
    if cfg!(target_os = "windows") {
        HostPlatform::Windows
    } else if cfg!(target_os = "macos") {
        HostPlatform::Macos
    } else {
        HostPlatform::Linux
    }
}

/// Determine user's home directory, honoring `HOME` env var override.
pub fn get_home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(dirs::home_dir)
}

/// Resolve a client configuration path for a specific host platform.
pub fn config_path_for(
    client: TargetClient,
    platform: HostPlatform,
    home: &Path,
    custom_path: Option<&Path>,
) -> Result<PathBuf> {
    if client == TargetClient::Custom {
        return custom_path
            .map(Path::to_path_buf)
            .ok_or_else(|| anyhow::anyhow!("Custom path must be provided via --path"));
    }
    let path = match (client, platform) {
        (TargetClient::ClaudeDesktop, HostPlatform::Macos) => {
            home.join("Library/Application Support/Claude/claude_desktop_config.json")
        }
        (TargetClient::ClaudeDesktop, HostPlatform::Windows) => {
            home.join("AppData/Roaming/Claude/claude_desktop_config.json")
        }
        (TargetClient::ClaudeDesktop, HostPlatform::Linux) => {
            home.join(".config/Claude/claude_desktop_config.json")
        }
        (TargetClient::GeminiAntigravity, _) => {
            home.join(".gemini/antigravity-cli/mcp/scientist-in-loop.json")
        }
        (TargetClient::Grok, _) => home.join(".grok/plugins/scientist-in-loop/.mcp.json"),
        (TargetClient::Cursor, _) => home.join(".cursor/mcp.json"),
        (TargetClient::OpenCode, HostPlatform::Windows) => {
            home.join("AppData/Roaming/opencode/opencode.json")
        }
        (TargetClient::OpenCode, _) => home.join(".config/opencode/opencode.json"),
        (TargetClient::Custom, _) => unreachable!(),
    };
    Ok(path)
}

/// Determine the target configuration file path using the current host.
pub fn get_config_path(client: TargetClient, custom_path: Option<&Path>) -> Result<PathBuf> {
    let home = get_home_dir().context("Could not determine user HOME directory")?;
    config_path_for(client, current_platform(), &home, custom_path)
}

/// Get the canonical path to the currently running `sil` binary.
pub fn get_sil_binary_path() -> String {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.canonicalize().ok().or(Some(p)))
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| "sil".to_string())
}

/// Options for MCP configuration lifecycle operations.
#[derive(Debug, Default, Clone)]
pub struct InstallOptions {
    pub client: Option<String>,
    pub path: Option<PathBuf>,
    pub project: Option<PathBuf>,
    pub hook: bool,
}

fn resolve_client(options: &InstallOptions) -> Result<(TargetClient, Option<PathBuf>)> {
    let client = match options.client.as_deref() {
        Some(value) => TargetClient::parse_str(value)?,
        None => {
            let selection = Select::new()
                .with_prompt("Select target AI client for sil MCP installation")
                .items(MENU_OPTIONS)
                .default(0)
                .interact()
                .context("Interactive selection failed (specify --client)")?;
            [
                TargetClient::GeminiAntigravity,
                TargetClient::Grok,
                TargetClient::ClaudeDesktop,
                TargetClient::Cursor,
                TargetClient::OpenCode,
                TargetClient::Custom,
            ][selection]
        }
    };
    let path = if client == TargetClient::Custom && options.path.is_none() {
        Some(PathBuf::from(
            Input::<String>::new()
                .with_prompt("Enter configuration file path")
                .interact()
                .context("Failed to read custom path")?,
        ))
    } else {
        options.path.clone()
    };
    Ok((client, path))
}

fn canonical_project(project: Option<&Path>) -> Result<String> {
    let cwd;
    let explicit = project.is_some();
    let path = if let Some(path) = project {
        path
    } else {
        cwd = std::env::current_dir().context("resolve MCP project root from current directory")?;
        &cwd
    };
    let canonical = path
        .canonicalize()
        .with_context(|| format!("MCP project root does not exist: {}", path.display()))?;
    if explicit && !canonical.join(".sil/config.yaml").is_file() {
        bail!("MCP project root is not a sil project: {}", path.display());
    }
    Ok(canonical.to_string_lossy().into_owned())
}

fn server_entry(client: TargetClient, project: &str) -> Value {
    let owner = json!({"managed_by": "scientist-in-loop", "version": 1});
    let args = if project.is_empty() {
        json!(["project", "mcp", "--quiet"])
    } else {
        json!(["project", "mcp", "--quiet", "--project", project])
    };
    match client {
        TargetClient::GeminiAntigravity if project.is_empty() => json!({
            "command": get_sil_binary_path(),
            "args": args,
            OWNER_KEY: owner,
        }),
        TargetClient::OpenCode => json!({
            "type": "local",
            "command": [get_sil_binary_path(), "project", "mcp", "--quiet", "--project", project],
            "enabled": true,
            "managed_by": "scientist-in-loop",
            OWNER_KEY: owner,
        }),
        _ => json!({
            "command": get_sil_binary_path(),
            "args": args,
            "managed_by": "scientist-in-loop",
            OWNER_KEY: owner,
        }),
    }
}

fn server_map(
    root: &mut Map<String, Value>,
    client: TargetClient,
) -> Result<&mut Map<String, Value>> {
    let key = if client == TargetClient::OpenCode {
        "mcp"
    } else {
        "mcpServers"
    };
    let value = root.entry(key).or_insert_with(|| Value::Object(Map::new()));
    value
        .as_object_mut()
        .ok_or_else(|| anyhow::anyhow!("MCP config field '{key}' must be an object"))
}

fn read_root(path: &Path) -> Result<(Value, bool)> {
    if !path.exists() {
        return Ok((json!({}), false));
    }
    let text =
        fs::read_to_string(path).with_context(|| format!("read MCP config {}", path.display()))?;
    let value: Value = serde_json::from_str(&text).with_context(|| {
        format!(
            "invalid JSON in MCP config {}; refusing to modify it",
            path.display()
        )
    })?;
    if !value.is_object() {
        bail!(
            "MCP config {} must contain a JSON object; refusing to modify it",
            path.display()
        );
    }
    Ok((value, true))
}

fn owned(value: &Value) -> bool {
    value
        .get(OWNER_KEY)
        .and_then(Value::as_object)
        .and_then(|m| m.get("managed_by"))
        == Some(&Value::String("scientist-in-loop".into()))
        || value.get("managed_by") == Some(&Value::String("scientist-in-loop".into()))
}

fn backup(path: &Path) -> Result<PathBuf> {
    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let backup = path.with_file_name(format!(
        "{}.sil-backup-{stamp}.json",
        path.file_name().unwrap().to_string_lossy()
    ));
    fs::copy(path, &backup)
        .with_context(|| format!("backup MCP config to {}", backup.display()))?;
    Ok(backup)
}

fn atomic_write(path: &Path, root: &Value) -> Result<()> {
    let utf8 = Utf8PathBuf::from_path_buf(path.to_path_buf())
        .map_err(|_| anyhow::anyhow!("MCP config path is not valid UTF-8"))?;
    sil_core::write_atomic_str(&utf8, &format!("{}\n", serde_json::to_string_pretty(root)?))?;
    Ok(())
}

/// Install or update an owned MCP entry.
pub fn install(options: InstallOptions) -> Result<PathBuf> {
    if options.hook {
        bail!("hooks are not supported for this host; no hook was installed");
    }
    let (client, custom) = resolve_client(&options)?;
    let path = match custom.clone() {
        Some(path) => path,
        None => get_config_path(client, None)?,
    };
    let project = canonical_project(options.project.as_deref())?;
    let (mut root, exists) = read_root(&path)?;
    if options.project.is_none() && client == TargetClient::GeminiAntigravity {
        let entry = server_entry(client, "");
        if exists {
            backup(&path)?;
        }
        atomic_write(&path, &entry)?;
        return Ok(path);
    }
    let map = server_map(root.as_object_mut().unwrap(), client)?;
    if let Some(current) = map.get(SERVER_NAME)
        && !owned(current)
    {
        bail!(
            "MCP entry '{SERVER_NAME}' in {} is not sil-owned; refusing to overwrite it",
            path.display()
        );
    }
    let entry = server_entry(
        client,
        if options.project.is_some() {
            &project
        } else {
            ""
        },
    );
    if map.get(SERVER_NAME) == Some(&entry) {
        return Ok(path);
    }
    map.insert(SERVER_NAME.into(), entry);
    if exists {
        backup(&path)?;
    }
    atomic_write(&path, &root)?;
    Ok(path)
}

/// Report whether the owned entry is installed.
pub fn status(options: InstallOptions) -> Result<InstallStatus> {
    let (client, custom) = resolve_client(&options)?;
    let path = match custom.clone() {
        Some(path) => path,
        None => get_config_path(client, None)?,
    };
    if !path.exists() {
        return Ok(InstallStatus {
            path,
            installed: false,
        });
    }
    let (mut root, _) = read_root(&path)?;
    let map = server_map(root.as_object_mut().unwrap(), client)?;
    Ok(InstallStatus {
        path,
        installed: map.get(SERVER_NAME).is_some_and(owned),
    })
}

/// Remove only the entry previously marked as sil-owned.
pub fn uninstall(options: InstallOptions) -> Result<PathBuf> {
    let (client, custom) = resolve_client(&options)?;
    let path = match custom.clone() {
        Some(path) => path,
        None => get_config_path(client, None)?,
    };
    if !path.exists() {
        return Ok(path);
    }
    let (mut root, _) = read_root(&path)?;
    let map = server_map(root.as_object_mut().unwrap(), client)?;
    match map.get(SERVER_NAME) {
        None => return Ok(path),
        Some(value) if !owned(value) => {
            bail!("MCP entry '{SERVER_NAME}' is not sil-owned; refusing to remove it")
        }
        Some(_) => {}
    }
    map.remove(SERVER_NAME);
    backup(&path)?;
    atomic_write(&path, &root)?;
    Ok(path)
}

/// Backwards-compatible installer entry point used by older command wiring.
#[allow(dead_code)]
pub fn run_installer(options: InstallOptions) -> Result<PathBuf> {
    install(options)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    fn project(dir: &Path) -> PathBuf {
        fs::create_dir_all(dir.join(".sil")).unwrap();
        fs::write(dir.join(".sil/config.yaml"), "version: 1\n").unwrap();
        dir.to_path_buf()
    }

    fn options(client: &str, path: &Path, root: &Path) -> InstallOptions {
        InstallOptions {
            client: Some(client.into()),
            path: Some(path.into()),
            project: Some(root.into()),
            hook: false,
        }
    }

    #[test]
    fn malformed_json_is_unchanged() {
        let d = tempdir().unwrap();
        let root = project(d.path().join("project").as_path());
        let path = d.path().join("config.json");
        fs::write(&path, "{broken").unwrap();
        assert!(install(options("custom", &path, &root)).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "{broken");
    }

    #[test]
    fn non_object_json_is_unchanged() {
        let d = tempdir().unwrap();
        let root = project(d.path().join("project").as_path());
        let path = d.path().join("config.json");
        fs::write(&path, "[]").unwrap();
        assert!(install(options("custom", &path, &root)).is_err());
        assert_eq!(fs::read_to_string(path).unwrap(), "[]");
    }

    #[test]
    fn preserves_fields_backups_and_is_idempotent() {
        let d = tempdir().unwrap();
        let root = project(d.path().join("project").as_path());
        let path = d.path().join("folder with spaces/config.json");
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            &path,
            r#"{"theme":"dark","mcpServers":{"other":{"command":"x"}}}"#,
        )
        .unwrap();
        let opts = options("custom", &path, &root);
        install(opts.clone()).unwrap();
        let first = fs::read_to_string(&path).unwrap();
        install(opts).unwrap();
        assert_eq!(first, fs::read_to_string(&path).unwrap());
        let json: Value = serde_json::from_str(&first).unwrap();
        assert_eq!(json["theme"], "dark");
        assert!(json["mcpServers"]["other"].is_object());
        assert!(fs::read_dir(path.parent().unwrap()).unwrap().any(|e| {
            e.unwrap()
                .file_name()
                .to_string_lossy()
                .contains("sil-backup")
        }));
    }

    #[test]
    fn uninstall_is_ownership_safe_and_opencode_uses_its_schema() {
        let d = tempdir().unwrap();
        let root = project(d.path().join("project").as_path());
        let path = d.path().join("open.json");
        install(options("opencode", &path, &root)).unwrap();
        assert!(status(options("opencode", &path, &root)).unwrap().installed);
        uninstall(options("opencode", &path, &root)).unwrap();
        assert!(!status(options("opencode", &path, &root)).unwrap().installed);
        fs::write(
            &path,
            r#"{"mcp":{"scientist-in-loop":{"command":["other"]}}}"#,
        )
        .unwrap();
        assert!(uninstall(options("opencode", &path, &root)).is_err());
    }

    #[test]
    fn platform_paths_cover_spaces_and_windows() {
        let home = Path::new("/tmp/home with spaces");
        assert!(
            config_path_for(
                TargetClient::ClaudeDesktop,
                HostPlatform::Windows,
                home,
                None
            )
            .unwrap()
            .to_string_lossy()
            .contains("AppData")
        );
        assert!(
            config_path_for(TargetClient::OpenCode, HostPlatform::Linux, home, None)
                .unwrap()
                .to_string_lossy()
                .contains(".config/opencode")
        );
    }
}
