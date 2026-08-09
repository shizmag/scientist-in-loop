//! Consolidated external API interactions and rate limiting for Scientist-in-Loop.

#![deny(missing_docs)]

/// ArXiv API lookups and BibTeX retrieval.
pub mod arxiv;
/// Crossref API metadata lookup and publication digest queries.
pub mod crossref;
/// DOI checking and BibTeX retrieval via content negotiation.
pub mod doi;
/// Error types for external API interactions.
pub mod error;
/// Global API rate limiting handler.
pub mod ratelimit;

#[cfg(test)]
mod tests;

pub use arxiv::*;
pub use crossref::*;
pub use doi::*;
pub use error::ApiError;
pub use ratelimit::*;
