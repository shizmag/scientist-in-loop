//! Domain types for manuscript health auditing and diagnostics.

use serde::{Deserialize, Serialize};

/// Severity level of a manuscript diagnostic issue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticLevel {
    /// Informational note (e.g. section word count).
    Info,
    /// Warning (e.g. unreferenced figure label, TODO item).
    Warning,
    /// Critical error (e.g. missing citation key).
    Error,
}

/// A single diagnostic finding from auditing `paper_draft.tex` / `references.bib`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HealthDiagnostic {
    /// Severity level.
    pub level: DiagnosticLevel,
    /// Category tag (e.g. "missing_citation", "unreferenced_label", "todo_item").
    pub category: String,
    /// Line number in `paper_draft.tex` if applicable.
    pub line: Option<usize>,
    /// Detailed diagnostic message.
    pub message: String,
}

/// Overall manuscript health audit report.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ManuscriptHealthReport {
    /// All diagnostics found during health audit.
    pub diagnostics: Vec<HealthDiagnostic>,
    /// Total word count of draft prose.
    pub word_count: usize,
    /// Count of missing citations.
    pub missing_citations_count: usize,
    /// Count of unreferenced figure/table labels.
    pub unreferenced_labels_count: usize,
    /// Count of `# -- X -- #` ideas or TODO blocks.
    pub todo_ideas_count: usize,
}

impl ManuscriptHealthReport {
    /// Has any critical errors.
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|d| d.level == DiagnosticLevel::Error)
    }

    /// Count of warnings.
    pub fn warning_count(&self) -> usize {
        self.diagnostics
            .iter()
            .filter(|d| d.level == DiagnosticLevel::Warning)
            .count()
    }
}
