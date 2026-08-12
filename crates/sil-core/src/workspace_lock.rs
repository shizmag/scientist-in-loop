//! Advisory workspace lock to coordinate agent/TUI writers.
//!
//! This is **not** a hard cross-process mutex. It records who last claimed an
//! operation so agents and humans can detect concurrent work. Last writer still
//! wins at the filesystem layer (same as TUI re-read-before-write).

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

use camino::{Utf8Path, Utf8PathBuf};
use serde::{Deserialize, Serialize};

use crate::error::SilError;
use crate::paths::{ProjectPaths, rel};

/// Advisory lock payload stored at [`.sil/workspace.lock`](rel::WORKSPACE_LOCK).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkspaceLock {
    /// Who holds the advisory claim (e.g. `mcp`, `tui`, `cli`).
    pub holder: String,
    /// Optional process id.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pid: Option<u32>,
    /// Unix seconds when the claim started.
    pub started: u64,
    /// Operation label (e.g. `edit-section`, `estimate`).
    pub op: String,
}

impl WorkspaceLock {
    /// Build a new advisory lock for the current process.
    pub fn new(holder: impl Into<String>, op: impl Into<String>) -> Self {
        Self {
            holder: holder.into(),
            pid: Some(std::process::id()),
            started: now_secs(),
            op: op.into(),
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Path to the advisory lock file for a project.
pub fn lock_path(paths: &ProjectPaths) -> Utf8PathBuf {
    paths.join(rel::WORKSPACE_LOCK)
}

/// Read the current advisory lock if present and parseable.
pub fn read_lock(paths: &ProjectPaths) -> Result<Option<WorkspaceLock>, SilError> {
    let path = lock_path(paths);
    if !path.is_file() {
        return Ok(None);
    }
    let text = fs::read_to_string(path.as_str())?;
    let lock: WorkspaceLock = serde_yaml::from_str(&text)
        .map_err(|e| SilError::Message(format!("parse {}: {e}", path)))?;
    Ok(Some(lock))
}

/// Write (overwrite) the advisory lock.
pub fn write_lock(paths: &ProjectPaths, lock: &WorkspaceLock) -> Result<(), SilError> {
    let path = lock_path(paths);
    let text = serde_yaml::to_string(lock)
        .map_err(|e| SilError::Message(format!("serialize workspace lock: {e}")))?;
    crate::atomic::write_atomic_str(&path, &text)?;
    Ok(())
}

/// Clear the advisory lock file if it exists.
pub fn clear_lock(paths: &ProjectPaths) -> Result<(), SilError> {
    let path = lock_path(paths);
    if path.is_file() {
        fs::remove_file(path.as_str())?;
    }
    Ok(())
}

/// True when another holder has a non-stale lock (default TTL 30 minutes).
pub fn is_busy(paths: &ProjectPaths, ttl_secs: u64) -> Result<Option<WorkspaceLock>, SilError> {
    match read_lock(paths)? {
        None => Ok(None),
        Some(lock) => {
            let age = now_secs().saturating_sub(lock.started);
            if age > ttl_secs {
                Ok(None)
            } else {
                Ok(Some(lock))
            }
        }
    }
}

/// Parse a lock from YAML string (test helper surface).
pub fn parse_lock_yaml(text: &str) -> Result<WorkspaceLock, SilError> {
    serde_yaml::from_str(text).map_err(|e| SilError::Message(format!("parse lock: {e}")))
}

/// Format lock as YAML.
pub fn lock_to_yaml(lock: &WorkspaceLock) -> Result<String, SilError> {
    serde_yaml::to_string(lock).map_err(|e| SilError::Message(format!("serialize lock: {e}")))
}

/// Ensure parent `.sil` exists for lock writes.
pub fn ensure_sil_dir(root: &Utf8Path) -> Result<(), SilError> {
    let sil = root.join(rel::SIL_DIR);
    fs::create_dir_all(sil.as_str())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn roundtrip_lock_file() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let paths = ProjectPaths::new(&root);
        fs::create_dir_all(paths.sil_dir().as_str()).unwrap();
        let lock = WorkspaceLock::new("mcp", "edit-section");
        write_lock(&paths, &lock).unwrap();
        let got = read_lock(&paths).unwrap().unwrap();
        assert_eq!(got.holder, "mcp");
        assert_eq!(got.op, "edit-section");
        assert!(got.pid.is_some());
        clear_lock(&paths).unwrap();
        assert!(read_lock(&paths).unwrap().is_none());
    }

    #[test]
    fn yaml_parse() {
        let y = "holder: tui\npid: 1\nstarted: 100\nop: hydrate\n";
        let lock = parse_lock_yaml(y).unwrap();
        assert_eq!(lock.holder, "tui");
        assert_eq!(lock.pid, Some(1));
    }
}
