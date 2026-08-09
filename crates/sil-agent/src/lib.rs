//! Dynamic skills loading and context generation for agents/humans.

#![deny(missing_docs)]

mod context;
mod error;
mod estimate;
mod paper;
mod skills;

pub use context::{ContextInput, generate_context, load_project_texts, sources_summary};
pub use error::ContextError;
pub use estimate::{
    EstimateDecision, EstimateDimensions, EstimateFinding, EstimateInput, EstimateMode,
    EstimateReport, estimate_proposal_message, report_to_markdown, run_heuristic_estimate,
    write_estimate_report,
};
pub use paper::{format_subsections_markdown, paper_subsections};
pub use skills::{ContextFlags, SkillSelection, load_skill};
