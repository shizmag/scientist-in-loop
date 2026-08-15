//! LaTeX engine abstraction and deterministic section splitting.

#![deny(missing_docs)]

mod archive;
mod compile;
mod engine;
mod error;
/// Comment-aware static manuscript dependency graph.
pub mod graph;
/// Manuscript health diagnostic checks.
pub mod health;
/// Parsing and updating inline LaTeX TODO / idea comment blocks.
pub mod idea_parser;
/// Staged, deterministic submission release packaging.
pub mod release;
mod sections;
mod split_write;

pub use archive::create_submission_archive;
pub use compile::{CompilerArtifact, CompilerResult, build, build_structured};
pub use engine::build_command;
pub use error::LatexError;
pub use graph::{
    AssetReference, CitationContext, DependencyKind, DependencyNode, DependencySnapshot,
    GraphOptions, LabelReference, build_dependency_graph,
};
pub use health::audit_manuscript;
pub use idea_parser::{
    format_idea_block, parse_idea_blocks, strip_idea_blocks, update_idea_block_status,
    update_or_insert_idea_block,
};
pub use release::{ReleaseOptions, create_staged_submission_release};
pub use sections::{TexSection, insert_cite_in_section, split_tex_sections};
pub use split_write::{
    WrittenSection, section_filename, slugify_title, write_draft_sections,
    write_draft_sections_from_file,
};
