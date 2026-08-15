//! Dynamic skills loading and context generation for agents/humans.

#![deny(missing_docs)]

mod context;
mod error;
pub mod estimate;
pub mod external;
mod paper;
mod registry;
mod skills;

pub use context::{ContextInput, generate_context, load_project_texts, sources_summary};
pub use error::ContextError;
pub use estimate::{
    EstimateDecision, EstimateDimensions, EstimateFinding, EstimateInput, EstimateMode,
    EstimateReport, estimate_proposal_message, report_to_markdown, run_heuristic_estimate,
    write_estimate_report,
};
pub use paper::{format_subsections_markdown, paper_subsections};
pub use registry::{
    CapabilityReport, CapabilityStatus, EntrypointCapabilityReport, ExternalDataFlow,
    HostCapabilities, InstalledSkill, SkillCapabilities, SkillDiff, SkillEntrypoint, SkillMetadata,
    SkillPackManifest, SkillRegistry, SkillRegistryError,
};
pub use skills::{ContextFlags, SkillSelection, load_skill};
