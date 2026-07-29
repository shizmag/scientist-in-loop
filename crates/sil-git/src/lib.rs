//! Git status, commit proposals, and Sci-Action trailers.

#![deny(missing_docs)]

mod cmd;
mod error;
mod log;
mod propose;
mod status;
mod trailers;

pub use error::GitError;
pub use log::{LogEntry, log_entries};
pub use propose::CommitProposal;
pub use status::{GitStatus, commit_all, init_repo, path_has_changes, repo_root, status};
pub use trailers::{SciAction, extract_from_message};
