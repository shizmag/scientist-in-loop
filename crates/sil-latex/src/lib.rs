//! LaTeX engine abstraction and deterministic section splitting.

#![deny(missing_docs)]

mod compile;
mod engine;
mod error;
pub mod health;
pub mod idea_parser;
mod sections;
mod split_write;

pub use compile::build;
pub use engine::build_command;
pub use error::LatexError;
pub use health::audit_manuscript;
pub use idea_parser::parse_idea_blocks;
pub use sections::{TexSection, split_tex_sections};
pub use split_write::{
    WrittenSection, section_filename, slugify_title, write_draft_sections,
    write_draft_sections_from_file,
};

