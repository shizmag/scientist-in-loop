use thiserror::Error;

/// Error type for external API interactions.
#[derive(Debug, Error, PartialEq, Eq, Clone)]
pub enum ApiError {
    /// Resource not found (e.g. HTTP 404).
    #[error("Not found: {0}")]
    NotFound(String),

    /// API rate limit exceeded (e.g. HTTP 429).
    #[error("Rate limited: {0}")]
    RateLimited(String),

    /// Network error or HTTP transport failure.
    #[error("Network error: {0}")]
    NetworkError(String),

    /// Parsing response JSON/XML failed.
    #[error("Parse error: {0}")]
    ParseError(String),

    /// Invalid input identifier (e.g. empty or malformed DOI).
    #[error("Invalid identifier: {0}")]
    InvalidIdentifier(String),
}
