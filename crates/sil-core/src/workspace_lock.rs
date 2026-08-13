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

/// Result of attempting to acquire or take an advisory workspace lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TakeLockResult {
    /// Lock was successfully acquired (missing, stale/dead PID, or already held by us).
    Acquired,
    /// Lock is currently held by another active process.
    Held(WorkspaceLock),
}

/// Alias for [`TakeLockResult`].
pub type TakeLock = TakeLockResult;

/// Check whether an operating system process with the given PID is currently alive.
///
/// On Unix platforms, this uses `kill(pid, 0)`. If the call returns 0, or fails with
/// `EPERM` (process exists but belongs to another user), the process is considered alive.
/// If it fails with `ESRCH` (or PID is 0 / invalid), the process is considered dead.
/// On non-Unix platforms, this returns `true` (safe best-effort fallback).
#[cfg(unix)]
pub fn pid_is_alive(pid: u32) -> bool {
    if pid == 0 || pid > i32::MAX as u32 {
        return false;
    }
    let res = unsafe { libc::kill(pid as libc::pid_t, 0) };
    if res == 0 {
        return true;
    }
    let errno = std::io::Error::last_os_error().raw_os_error();
    errno == Some(libc::EPERM)
}

/// Check whether an operating system process with the given PID is currently alive (fallback).
#[cfg(not(unix))]
pub fn pid_is_alive(_pid: u32) -> bool {
    // Best-effort on non-unix platforms (assumes alive to be safe).
    true
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
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent.as_str())?;
    }
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

/// Acquire the lock if missing or stale (dead PID), or refresh if held by the same holder and PID.
/// Otherwise returns [`TakeLockResult::Held`].
pub fn take_or_stale(
    paths: &ProjectPaths,
    new_lock: &WorkspaceLock,
) -> Result<TakeLockResult, SilError> {
    ensure_sil_dir(&paths.root)?;
    match read_lock(paths)? {
        None => {
            write_lock(paths, new_lock)?;
            Ok(TakeLockResult::Acquired)
        }
        Some(existing) => {
            let is_alive = existing.pid.map(pid_is_alive).unwrap_or(true);
            if !is_alive {
                // Dead PID: clear stale lock and acquire.
                clear_lock(paths)?;
                write_lock(paths, new_lock)?;
                Ok(TakeLockResult::Acquired)
            } else if existing.pid == new_lock.pid && existing.holder == new_lock.holder {
                // Already held by us: refresh lock.
                write_lock(paths, new_lock)?;
                Ok(TakeLockResult::Acquired)
            } else {
                // Held by another live process/holder.
                Ok(TakeLockResult::Held(existing))
            }
        }
    }
}

/// Attempt to acquire an advisory lock for the current process.
pub fn try_acquire_lock(
    paths: &ProjectPaths,
    holder: &str,
    op: &str,
) -> Result<TakeLockResult, SilError> {
    let new_lock = WorkspaceLock::new(holder, op);
    take_or_stale(paths, &new_lock)
}

/// Attempt to acquire an advisory lock given a project root directory.
pub fn try_acquire_lock_root(
    root: &Utf8Path,
    holder: &str,
    op: &str,
) -> Result<TakeLockResult, SilError> {
    let paths = ProjectPaths::new(root);
    try_acquire_lock(&paths, holder, op)
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

    #[test]
    fn test_pid_is_alive() {
        assert!(pid_is_alive(std::process::id()));
        assert!(!pid_is_alive(0));
        assert!(!pid_is_alive(99_999_999));
        assert!(!pid_is_alive(i32::MAX as u32));
    }

    #[test]
    fn test_take_or_stale_missing_lock() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let paths = ProjectPaths::new(&root);

        let res = try_acquire_lock(&paths, "tui", "session").unwrap();
        assert_eq!(res, TakeLockResult::Acquired);

        let current = read_lock(&paths).unwrap().unwrap();
        assert_eq!(current.holder, "tui");
        assert_eq!(current.op, "session");
        assert_eq!(current.pid, Some(std::process::id()));
    }

    #[test]
    fn test_take_or_stale_dead_pid_is_taken() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let paths = ProjectPaths::new(&root);

        // Pre-populate with a dead PID
        let stale = WorkspaceLock {
            holder: "mcp".to_string(),
            pid: Some(99_999_999),
            started: 100,
            op: "edit-section".to_string(),
        };
        write_lock(&paths, &stale).unwrap();

        // Attempt acquire
        let res = try_acquire_lock(&paths, "tui", "session").unwrap();
        assert_eq!(res, TakeLockResult::Acquired);

        let current = read_lock(&paths).unwrap().unwrap();
        assert_eq!(current.holder, "tui");
        assert_eq!(current.pid, Some(std::process::id()));
    }

    #[test]
    fn test_take_or_stale_live_other_holder_is_held() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let paths = ProjectPaths::new(&root);

        // Simulate a lock held by "mcp" using the current process PID (which is alive)
        let live_other = WorkspaceLock {
            holder: "mcp".to_string(),
            pid: Some(std::process::id()),
            started: 100,
            op: "estimate".to_string(),
        };
        write_lock(&paths, &live_other).unwrap();

        let res = try_acquire_lock(&paths, "tui", "session").unwrap();
        match res {
            TakeLockResult::Held(held) => {
                assert_eq!(held.holder, "mcp");
                assert_eq!(held.op, "estimate");
            }
            TakeLockResult::Acquired => panic!("Expected Held, got Acquired"),
        }
    }

    #[test]
    fn test_take_or_stale_same_holder_and_pid_refreshes() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let paths = ProjectPaths::new(&root);

        let initial = WorkspaceLock::new("tui", "session");
        write_lock(&paths, &initial).unwrap();

        let res = try_acquire_lock(&paths, "tui", "save_all").unwrap();
        assert_eq!(res, TakeLockResult::Acquired);

        let current = read_lock(&paths).unwrap().unwrap();
        assert_eq!(current.holder, "tui");
        assert_eq!(current.op, "save_all");
    }
}
