//! LaTeX build errors.

use sil_core::SilError;
use thiserror::Error;

/// LaTeX build errors.
#[derive(Debug, Error)]
pub enum LatexError {
    /// Engine binary not found.
    #[error(
        "LaTeX engine '{engine}' not found on PATH; install it or change latex.engine in config"
    )]
    EngineNotFound {
        /// Engine name.
        engine: String,
    },
    /// Compilation failed.
    #[error("LaTeX build failed ({engine}): {message}")]
    BuildFailed {
        /// Engine name.
        engine: String,
        /// Error detail.
        message: String,
    },
    /// Main file missing.
    #[error("main LaTeX file not found: {0}")]
    MainNotFound(String),
    /// Other.
    #[error("{0}")]
    Message(String),
}

impl From<LatexError> for SilError {
    fn from(value: LatexError) -> Self {
        SilError::Build(value.to_string())
    }
}
