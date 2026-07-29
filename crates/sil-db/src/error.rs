//! Database errors.

use sil_core::SilError;
use thiserror::Error;

/// Database-specific errors.
#[derive(Debug, Error)]
pub enum DbError {
    /// SQLite error.
    #[error("sqlite: {0}")]
    Sqlite(#[from] rusqlite::Error),
    /// Domain message.
    #[error("{0}")]
    Message(String),
}

impl From<DbError> for SilError {
    fn from(value: DbError) -> Self {
        SilError::Database(value.to_string())
    }
}
