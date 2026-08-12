//! Application use-case layer for `sil`.
//!
//! Provides sync use-case operations for CLI, TUI, and MCP surfaces.

#![deny(missing_docs)]

mod bib;
mod context;
mod error;

pub use bib::{PromoteBib, PromoteBibResult, UpsertBib, UpsertBibResult, promote_bib, upsert_bib};
pub use context::AppContext;
pub use error::AppError;
