//! Core domain types, configuration, errors, paths, and validation for `sil`.
//!
//! Stage 0: domain types and path helpers. Later stages add config/structure loaders.

#![deny(missing_docs)]

mod bib;
mod config;
mod error;
/// Project path constants and helpers.
pub mod paths;
mod sci_action;
mod source;
mod stage;
mod structure;
mod terminal;
mod types;

/// Global/local settings and cache.
pub mod settings;

pub mod digest;
pub mod health;
pub mod todo;

pub use bib::{
    BibSuggestion, format_bibtex_article, format_cite_command, slug_cite_key, suggest_from_query,
    suggest_from_reference_entry, suggest_from_source,
};
pub use config::{Config, LatexConfig, ParsingConfig, PathsConfig, ProjectConfig};
pub use digest::JournalPublication;
pub use error::{ConfigError, SilError, StructureError, ValidationError};
pub use health::{DiagnosticLevel, HealthDiagnostic, ManuscriptHealthReport};
pub use paths::{ProjectPaths, project_root_from_cwd};
pub use sci_action::{SciAction, extract_from_message};
pub use settings::{
    AuthorDetails, GlobalSettings, GrantDetails, LocalSettings, RagSettings, SettingsCache,
};
pub use source::{DocumentStatus, ReferenceEntry, SourceDocument, SourceId, validate_pdf_path};
pub use stage::Stage;
pub use structure::{CompletionSummary, Section, SectionCompletion, Structure};
pub use terminal::{NullUi, ProgressHandle, SilUi, SpinnerHandle, StdUi};
pub use todo::{IdeaBlock, TodoIdea};
pub use types::{LatexEngine, PaperKind, SilProject};

