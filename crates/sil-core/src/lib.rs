//! Core domain types, configuration, errors, paths, and validation for `sil`.
//!
//! Stage 0: domain types and path helpers. Later stages add config/structure loaders.

#![deny(missing_docs)]

/// Bibliography & BibTeX utilities.
pub mod bib;
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

/// Crash-safe atomic file writing.
pub mod atomic;
/// Journal publication digest entries.
pub mod digest;
/// Manuscript health check diagnostic tools.
pub mod health;
/// Project TODO and idea tracking blocks.
pub mod todo;
/// Advisory workspace lock for agent/TUI coordination.
pub mod workspace_lock;

pub use atomic::{write_atomic, write_atomic_str};
pub use bib::{
    BibEntryInfo, BibSuggestion, TUI_ADDED_MARKER, UpsertOptions, extract_bib_entry_info,
    format_bibtex_article, format_cite_command, is_same_paper, is_tui_added_bib_block,
    mark_tui_added_bib_entry, normalize_arxiv_id, parse_bib_blocks, pretty_format_bibtex,
    rewrite_bib_cite_key, slug_cite_key, strip_tui_added_bib_entries, suggest_from_filename_title,
    suggest_from_query, suggest_from_reference_entry, suggest_from_source,
    unmark_tui_added_bib_entry, upsert_bib_entry, upsert_bib_entry_with_options,
};
pub use config::{Config, LatexConfig, ParsingConfig, PathsConfig, ProjectConfig};
pub use digest::JournalPublication;
pub use error::{ConfigError, SilError, StructureError, UserError, ValidationError};
pub use health::{DiagnosticLevel, HealthDiagnostic, ManuscriptHealthReport};
pub use paths::{ProjectPaths, project_root_from_cwd};
pub use sci_action::{SciAction, extract_from_message};
pub use settings::{
    AuthorDetails, GlobalSettings, GrantDetails, LocalSettings, RagSettings, SettingsCache,
    effective_digest_query, effective_digest_refresh_hours,
};
pub use source::{
    DocumentStatus, ReferenceEntry, SourceDocument, SourceId, SourceKind, compute_draft_hash,
    probe_source, ref_text_for_embed, should_attempt_metadata_fetch,
    should_attempt_metadata_fetch_source, strip_latex_for_embed, validate_pdf_path,
};
pub use stage::Stage;
pub use structure::{CompletionSummary, Section, SectionCompletion, Structure};
pub use terminal::{NullUi, ProgressHandle, SilUi, SpinnerHandle, StdUi};
pub use todo::{IdeaBlock, TodoIdea};
pub use types::{LatexEngine, PaperKind, SilProject};
pub use workspace_lock::{
    TakeLock, TakeLockResult, WorkspaceLock, clear_lock, is_busy, lock_path, lock_to_yaml,
    parse_lock_yaml, pid_is_alive, read_lock, take_or_stale, try_acquire_lock,
    try_acquire_lock_root, write_lock,
};
