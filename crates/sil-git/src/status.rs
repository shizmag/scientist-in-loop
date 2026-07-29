//! Working-tree status helpers.

use camino::{Utf8Path, Utf8PathBuf};

use crate::cmd::run_git;
use crate::error::GitError;
use crate::propose::CommitProposal;

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
            let path = e.get(3..).unwrap_or(e.as_str()).trim();
            let path = path.rsplit_once(" -> ").map(|(_, n)| n).unwrap_or(path);
            path == rel || path.ends_with(rel) || path.contains(rel)
        })
    }
}

/// Initialize a git repository at `root` if one does not exist.
pub fn init_repo(root: &Utf8Path) -> Result<(), GitError> {
    if root.join(".git").exists() {
        return Ok(());
    }
    run_git(root, &["init"])?;
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

/// Stage all and create a real commit (tests / explicit tools only).
/// Production CLI never auto-commits; it only proposes.
pub fn commit_all(root: &Utf8Path, proposal: &CommitProposal) -> Result<String, GitError> {
    run_git(root, &["add", "-A"])?;
    let msg = proposal.message();
    run_git(root, &["commit", "-m", &msg])
}

/// Whether a relative path has uncommitted changes.
pub fn path_has_changes(root: &Utf8Path, rel: &str) -> Result<bool, GitError> {
    let st = status(root)?;
    if !st.is_repo {
        return Ok(false);
    }
    Ok(st.path_dirty(rel))
}

/// Resolve repo root.
pub fn repo_root(start: &Utf8Path) -> Result<Utf8PathBuf, GitError> {
    let out = run_git(start, &["rev-parse", "--show-toplevel"])?;
    Ok(Utf8PathBuf::from(out.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

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

    #[test]
    fn status_not_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let st = status(&root).unwrap();
        assert!(!st.is_repo);
        assert!(st.clean);
        assert!(!path_has_changes(&root, "x").unwrap());
    }

    #[test]
    fn init_idempotent() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        init_repo(&root).unwrap();
        assert!(root.join(".git").exists());
    }

    #[test]
    fn commit_all_and_clean_tree() {
        use crate::propose::CommitProposal;
        use sil_core::SciAction;

        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        std::fs::write(root.join("file.txt"), "data").unwrap();
        let p = CommitProposal::new("Add file", SciAction::AddData);
        commit_all(&root, &p).unwrap();
        let st = status(&root).unwrap();
        assert!(st.clean);
        assert!(!path_has_changes(&root, "file.txt").unwrap());
    }

    #[test]
    fn path_dirty_rename_style_entry() {
        let mut st = GitStatus {
            is_repo: true,
            entries: vec!["R  old.txt -> new.txt".into()],
            clean: false,
        };
        assert!(st.path_dirty("new.txt"));
        st.entries = vec!["?? sources/paper.pdf".into()];
        assert!(st.path_dirty("sources/paper.pdf"));
        assert!(st.path_dirty("paper.pdf"));
    }

    #[test]
    fn clean_working_tree_after_commit() {
        use crate::propose::CommitProposal;
        use sil_core::SciAction;

        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        std::fs::write(root.join("a.txt"), "1").unwrap();
        assert!(!status(&root).unwrap().clean);
        commit_all(&root, &CommitProposal::new("c", SciAction::Init)).unwrap();
        let st = status(&root).unwrap();
        assert!(st.clean);
        assert!(st.entries.is_empty());
    }

    #[test]
    fn dirty_tree_reports_untracked() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        std::fs::write(root.join("dirty.txt"), "x").unwrap();
        let st = status(&root).unwrap();
        assert!(!st.clean);
        assert!(st.entries.iter().any(|e| e.contains("dirty.txt")));
        assert!(path_has_changes(&root, "dirty.txt").unwrap());
    }
}
