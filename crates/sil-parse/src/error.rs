//! Parse errors.

use sil_core::SilError;
use thiserror::Error;

/// Parse-related errors.
#[derive(Debug, Error)]
pub enum ParseError {
    /// Document validation failed.
    #[error("{0}")]
    InvalidDocument(String),
    /// Python / Marker invocation failed.
    #[error("Marker parse failed: {0}")]
    Marker(String),
    /// Database write failed.
    #[error("database: {0}")]
    Db(String),
    /// Other.
    #[error("{0}")]
    Message(String),
}

impl From<ParseError> for SilError {
    fn from(value: ParseError) -> Self {
        SilError::Parse(value.to_string())
    }
}
