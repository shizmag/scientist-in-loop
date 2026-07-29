//! Git errors.

use sil_core::SilError;
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
