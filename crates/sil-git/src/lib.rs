//! Git status, commit proposals, and Sci-Action trailers.
//!
//! Stage 0: skeleton with proposal formatting.
//! Stage 4: full git integration via `git` CLI.

#![deny(missing_docs)]

use std::process::Command;

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::{SciAction, SilError};
use thiserror::Error;

/// Git-related errors.
#[derive(Debug, Error)]
pub enum GitError {
    /// Git is not installed or not on PATH.
    #[error("git executable not found; install git to use version-control features")]
    NotFound,
    /// Command failed.
    #[error("git {command} failed: {stderr}")]
    CommandFailed {
        /// Subcommand name.
        command: String,
        /// Combined stderr/stdout.
        stderr: String,
    },
    /// Not a git repository.
    #[error("not a git repository at {0}")]
    NotARepo(String),
    /// Other.
    #[error("{0}")]
    Message(String),
}

impl From<GitError> for SilError {
    fn from(value: GitError) -> Self {
        SilError::Git(value.to_string())
    }
}

/// A proposed (not applied) commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitProposal {
    /// Subject line.
    pub subject: String,
    /// Optional body paragraphs.
    pub body: Vec<String>,
    /// Sci-Action trailer.
    pub action: SciAction,
}

impl CommitProposal {
    /// Create a proposal with subject and Sci-Action.
    pub fn new(subject: impl Into<String>, action: SciAction) -> Self {
        Self {
            subject: subject.into(),
            body: Vec::new(),
            action,
        }
    }

    /// Add a body paragraph.
    pub fn with_body(mut self, paragraph: impl Into<String>) -> Self {
        self.body.push(paragraph.into());
        self
    }

    /// Full commit message including Sci-Action trailer.
    pub fn message(&self) -> String {
        let mut msg = self.subject.clone();
        if !self.body.is_empty() {
            msg.push_str("\n\n");
            msg.push_str(&self.body.join("\n\n"));
        }
        msg.push_str("\n\n");
        msg.push_str(&self.action.trailer_line());
        msg.push('\n');
        msg
    }

    /// Human-readable proposal block for the terminal.
    pub fn display(&self) -> String {
        format!(
            "Proposed commit (not applied):\n---\n{}---\nRun: git add -A && git commit -F - <<'EOF'\n{}EOF",
            self.message(),
            self.message()
        )
    }
}

/// Snapshot of git working tree state.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GitStatus {
    /// Whether `.git` exists.
    pub is_repo: bool,
    /// Porcelain short status lines.
    pub entries: Vec<String>,
    /// Whether the working tree is clean.
    pub clean: bool,
}

impl GitStatus {
    /// True if `path` (relative to repo) has uncommitted changes.
    pub fn path_dirty(&self, rel: &str) -> bool {
        let rel = rel.trim_start_matches("./");
        self.entries.iter().any(|e| {
            // porcelain v1: XY path
            let path = e.get(3..).unwrap_or(e.as_str()).trim();
            // handle renames "old -> new"
            let path = path.rsplit_once(" -> ").map(|(_, n)| n).unwrap_or(path);
            path == rel || path.ends_with(rel) || path.contains(rel)
        })
    }
}

/// One annotated log entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LogEntry {
    /// Short hash.
    pub hash: String,
    /// Subject.
    pub subject: String,
    /// Parsed Sci-Action if present.
    pub action: Option<SciAction>,
}

fn git_cmd() -> Result<Command, GitError> {
    // Prefer `git` on PATH.
    Ok(Command::new("git"))
}

fn run_git(repo: &Utf8Path, args: &[&str]) -> Result<String, GitError> {
    let mut cmd = git_cmd()?;
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

/// Initialize a git repository at `root` if one does not exist.
pub fn init_repo(root: &Utf8Path) -> Result<(), GitError> {
    if root.join(".git").exists() {
        return Ok(());
    }
    run_git(root, &["init"])?;
    // Reasonable local defaults for empty identity in tests/CI.
    let _ = run_git(root, &["config", "user.email", "sil@localhost"]);
    let _ = run_git(root, &["config", "user.name", "sil"]);
    Ok(())
}

/// Collect working tree status.
pub fn status(root: &Utf8Path) -> Result<GitStatus, GitError> {
    if !root.join(".git").exists() {
        return Ok(GitStatus {
            is_repo: false,
            entries: Vec::new(),
            clean: true,
        });
    }
    let out = run_git(root, &["status", "--porcelain"])?;
    let entries: Vec<String> = out
        .lines()
        .map(str::trim_end)
        .filter(|l| !l.is_empty())
        .map(str::to_string)
        .collect();
    let clean = entries.is_empty();
    Ok(GitStatus {
        is_repo: true,
        entries,
        clean,
    })
}

/// Stage all and create a real commit (used only when tests explicitly need history).
/// Production CLI never auto-commits; it only proposes.
pub fn commit_all(root: &Utf8Path, proposal: &CommitProposal) -> Result<String, GitError> {
    run_git(root, &["add", "-A"])?;
    let msg = proposal.message();
    let out = run_git(root, &["commit", "-m", &msg])?;
    Ok(out)
}

/// Read log entries, optionally only those with Sci-Action trailers.
pub fn log_entries(root: &Utf8Path, limit: usize, only_sci: bool) -> Result<Vec<LogEntry>, GitError> {
    if !root.join(".git").exists() {
        return Err(GitError::NotARepo(root.to_string()));
    }
    // %H full hash, %h short, %s subject, %b body
    let out = match run_git(
        root,
        &[
            "log",
            &format!("-n{limit}"),
            "--format=%h%x1f%s%x1f%b%x1e",
        ],
    ) {
        Ok(o) => o,
        Err(GitError::CommandFailed { stderr, .. })
            if stderr.contains("does not have any commits")
                || stderr.contains("unknown revision")
                || stderr.contains("bad default revision") =>
        {
            return Ok(Vec::new());
        }
        Err(e) => return Err(e),
    };

    let mut entries = Vec::new();
    for record in out.split('\x1e') {
        let record = record.trim();
        if record.is_empty() {
            continue;
        }
        let mut parts = record.splitn(3, '\x1f');
        let hash = parts.next().unwrap_or("").trim().to_string();
        let subject = parts.next().unwrap_or("").trim().to_string();
        let body = parts.next().unwrap_or("").trim();
        let action = sil_core::extract_from_message(&format!("{subject}\n\n{body}"));
        if only_sci && action.is_none() {
            continue;
        }
        entries.push(LogEntry {
            hash,
            subject,
            action,
        });
    }
    Ok(entries)
}

/// Whether a relative path has uncommitted changes.
pub fn path_has_changes(root: &Utf8Path, rel: &str) -> Result<bool, GitError> {
    let st = status(root)?;
    if !st.is_repo {
        return Ok(false);
    }
    Ok(st.path_dirty(rel))
}

/// Resolve repo root (may equal project root after init).
pub fn repo_root(start: &Utf8Path) -> Result<Utf8PathBuf, GitError> {
    let out = run_git(start, &["rev-parse", "--show-toplevel"])?;
    let p = out.trim();
    Ok(Utf8PathBuf::from(p))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sil_core::SciAction;

    #[test]
    fn proposal_includes_trailer() {
        let p = CommitProposal::new("Initialize sil project", SciAction::Init)
            .with_body("Created workspace layout and database.");
        let msg = p.message();
        assert!(msg.contains("Initialize sil project"));
        assert!(msg.contains("Sci-Action: init"));
        assert_eq!(
            sil_core::extract_from_message(&msg),
            Some(SciAction::Init)
        );
    }

    #[test]
    fn parse_pdf_trailer() {
        let p = CommitProposal::new("Parse source PDF", SciAction::ParsePdf);
        assert!(p.message().contains("Sci-Action: parse-pdf"));
    }

    #[test]
    fn init_and_status() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        std::fs::write(root.join("README.md"), "hi").unwrap();
        let st = status(&root).unwrap();
        assert!(st.is_repo);
        assert!(!st.clean);
        assert!(st.path_dirty("README.md"));
    }
}
