//! Application use-case layer for `sil`.
//!
//! Provides sync use-case operations for CLI, TUI, and MCP surfaces.

#![deny(missing_docs)]

mod bib;
mod context;
pub mod doctor;
mod error;
mod fetch;
pub mod init;
pub mod templates;

pub use bib::{PromoteBib, PromoteBibResult, UpsertBib, UpsertBibResult, promote_bib, upsert_bib};
pub use context::AppContext;
pub use doctor::{DatabaseRepairReport, SourceRepairOutcome, repair_sqlite_database};
pub use error::AppError;
pub use fetch::{FetchSource, FetchSourceResult, ParseSummary, fetch_source};
pub use init::{init_project, update_project};

