//! Context / skill errors.

use sil_core::SilError;
use thiserror::Error;

/// Context generation errors.
#[derive(Debug, Error)]
pub enum ContextError {
    /// I/O failure.
    #[error("I/O: {0}")]
    Io(String),
    /// Missing required skill.
    #[error("missing skill file: {0}")]
    MissingSkill(String),
    /// Other.
    #[error("{0}")]
    Message(String),
}

impl From<ContextError> for SilError {
    fn from(value: ContextError) -> Self {
        SilError::Message(value.to_string())
    }
}
