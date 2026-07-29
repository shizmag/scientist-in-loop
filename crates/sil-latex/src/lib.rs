//! LaTeX engine abstraction and deterministic section splitting.

#![deny(missing_docs)]

mod compile;
mod engine;
mod error;
mod sections;

pub use compile::build;
pub use engine::build_command;
pub use error::LatexError;
pub use sections::{TexSection, split_tex_sections};
