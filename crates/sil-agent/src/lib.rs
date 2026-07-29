//! Dynamic skills loading and context generation for agents/humans.

#![deny(missing_docs)]

mod context;
mod error;
mod paper;
mod skills;

pub use context::{ContextInput, generate_context, load_project_texts, sources_summary};
pub use error::ContextError;
pub use paper::{format_subsections_markdown, paper_subsections};
pub use skills::{ContextFlags, SkillSelection, load_skill};
