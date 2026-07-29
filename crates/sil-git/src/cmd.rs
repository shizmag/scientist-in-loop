//! Low-level `git` process invocation.

use std::process::Command;

use camino::Utf8Path;

use crate::error::GitError;

pub(crate) fn run_git(repo: &Utf8Path, args: &[&str]) -> Result<String, GitError> {
    let mut cmd = Command::new("git");
    let output = cmd
        .args(args)
        .current_dir(repo.as_str())
        .output()
        .map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                GitError::NotFound
            } else {
                GitError::Message(e.to_string())
            }
        })?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
        return Err(GitError::CommandFailed {
            command: args.first().unwrap_or(&"").to_string(),
            stderr: if stderr.is_empty() { stdout } else { stderr },
        });
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
