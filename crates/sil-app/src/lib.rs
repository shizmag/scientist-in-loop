//! Application use-case layer for `sil`.
//!
//! Provides sync use-case operations for CLI, TUI, and MCP surfaces.

#![deny(missing_docs)]

mod bib;
mod check;
mod context;
pub mod discovery;
pub mod doctor;
mod error;
mod fetch;
pub mod init;
pub mod template_packs;
pub mod templates;

pub use bib::{PromoteBib, PromoteBibResult, UpsertBib, UpsertBibResult, promote_bib, upsert_bib};
pub use check::{ManuscriptCheckOptions, load_cached_report, run_manuscript_check};
pub use context::AppContext;
pub use discovery::{
    CandidateForRanking, DiscoveryOptions, DiscoveryResult, IdentityDecision, IdentityRelation,
    RankedCandidate, RankingComponents, discover_candidates, identify, rank_and_store,
    rank_candidates, transition_candidate,
};
pub use doctor::{DatabaseRepairReport, SourceRepairOutcome, repair_sqlite_database};
pub use error::AppError;
pub use fetch::{FetchSource, FetchSourceResult, ParseSummary, fetch_source};
pub use init::{init_project, update_project};
