//! Annotated git log with Sci-Action trailers.

use camino::Utf8Path;
use sil_core::SciAction;

use crate::cmd::run_git;
use crate::error::GitError;
use crate::trailers::extract_from_message;

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

/// Read log entries, optionally only those with Sci-Action trailers.
pub fn log_entries(
    root: &Utf8Path,
    limit: usize,
    only_sci: bool,
) -> Result<Vec<LogEntry>, GitError> {
    if !root.join(".git").exists() {
        return Err(GitError::NotARepo(root.to_string()));
    }
    let out = match run_git(
        root,
        &["log", &format!("-n{limit}"), "--format=%h%x1f%s%x1f%b%x1e"],
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
        let action = extract_from_message(&format!("{subject}\n\n{body}"));
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::propose::CommitProposal;
    use crate::status::{commit_all, init_repo};
    use camino::Utf8PathBuf;
    use sil_core::SciAction;

    #[test]
    fn log_entries_not_a_repo() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        assert!(matches!(
            log_entries(&root, 10, true),
            Err(GitError::NotARepo(_))
        ));
    }

    #[test]
    fn log_empty_repo() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        let entries = log_entries(&root, 10, false).unwrap();
        assert!(entries.is_empty());
    }

    #[test]
    fn log_filters_sci_action() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        std::fs::write(root.join("a.txt"), "1").unwrap();
        commit_all(&root, &CommitProposal::new("with trailer", SciAction::Init)).unwrap();
        std::fs::write(root.join("b.txt"), "2").unwrap();
        // plain commit without trailer
        crate::cmd::run_git(&root, &["add", "-A"]).unwrap();
        crate::cmd::run_git(&root, &["commit", "-m", "no trailer here"]).unwrap();

        let all = log_entries(&root, 10, false).unwrap();
        assert_eq!(all.len(), 2);
        let sci = log_entries(&root, 10, true).unwrap();
        assert_eq!(sci.len(), 1);
        assert_eq!(sci[0].action, Some(SciAction::Init));
        assert!(sci[0].subject.contains("with trailer"));
        assert!(!sci[0].hash.is_empty());
    }

    #[test]
    fn log_limit_one() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        for i in 0..3 {
            std::fs::write(root.join(format!("f{i}.txt")), "x").unwrap();
            commit_all(
                &root,
                &CommitProposal::new(format!("c{i}"), SciAction::EditDraft),
            )
            .unwrap();
        }
        let entries = log_entries(&root, 1, false).unwrap();
        assert_eq!(entries.len(), 1);
    }

    #[test]
    fn log_parses_fetch_source_trailer() {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        init_repo(&root).unwrap();
        std::fs::write(root.join("x.txt"), "1").unwrap();
        commit_all(&root, &CommitProposal::new("fetch", SciAction::FetchSource)).unwrap();
        let sci = log_entries(&root, 5, true).unwrap();
        assert_eq!(sci[0].action, Some(SciAction::FetchSource));
    }
}
