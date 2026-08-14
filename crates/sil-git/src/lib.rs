//! Git status, commit proposals, and Sci-Action trailers.

#![deny(missing_docs)]

mod action_infer;
mod cmd;
mod error;
mod log;
mod propose;
mod status;
mod trailers;

pub use action_infer::{
    dirty_paths, infer_action_from_paths, proposal_for_action, propose_from_status,
};
pub use error::GitError;
pub use log::{LogEntry, log_entries};
pub use propose::CommitProposal;
pub use status::{
    GitStatus, commit_all, diff_for_paths, init_repo, path_has_changes, repo_root, status,
};
pub use trailers::{SciAction, extract_from_message};
