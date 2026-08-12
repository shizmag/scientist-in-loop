//! Error types for the application layer.

use thiserror::Error;

/// Errors emitted by `sil-app` use-case functions.
#[derive(Debug, Error)]
pub enum AppError {
    /// Current directory or specified path is not inside a sil project.
    #[error("not a sil project (missing .sil/config.yaml); run `sil init` first")]
    NotInProject,

    /// Target file or entity not found.
    #[error("{0}")]
    NotFound(String),

    /// Invalid BibTeX content or request arguments.
    #[error("invalid BibTeX: {0}")]
    InvalidBib(String),

    /// I/O error occurred.
    #[error("I/O error at {path}: {source}")]
    Io {
        /// File path associated with the error.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },

    /// Generic error message.
    #[error("{0}")]
    Message(String),
}
