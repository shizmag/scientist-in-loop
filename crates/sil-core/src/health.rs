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
    /// Total bib keys defined in references.bib.
    pub total_bib_keys_count: usize,
    /// Count of unique bib keys defined in references.bib that are mentioned in paper_*.tex.
    pub cited_bib_keys_count: usize,
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

    /// Ratio of (cited_bib_keys_count, total_bib_keys_count).
    pub fn bib_citation_ratio(&self) -> (usize, usize) {
        (self.cited_bib_keys_count, self.total_bib_keys_count)
    }

    /// Count of bib keys defined in references.bib that are NOT mentioned in paper_*.tex.
    pub fn unmentioned_bib_keys_count(&self) -> usize {
        self.total_bib_keys_count.saturating_sub(self.cited_bib_keys_count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnostic_level_serde() {
        let yaml = serde_yaml::to_string(&DiagnosticLevel::Info).unwrap();
        assert_eq!(yaml.trim(), "info");
        let yaml = serde_yaml::to_string(&DiagnosticLevel::Warning).unwrap();
        assert_eq!(yaml.trim(), "warning");
        let yaml = serde_yaml::to_string(&DiagnosticLevel::Error).unwrap();
        assert_eq!(yaml.trim(), "error");

        let de: DiagnosticLevel = serde_yaml::from_str("info").unwrap();
        assert_eq!(de, DiagnosticLevel::Info);
        let de: DiagnosticLevel = serde_yaml::from_str("warning").unwrap();
        assert_eq!(de, DiagnosticLevel::Warning);
        let de: DiagnosticLevel = serde_yaml::from_str("error").unwrap();
        assert_eq!(de, DiagnosticLevel::Error);
    }

    #[test]
    fn health_diagnostic_serde() {
        let diag = HealthDiagnostic {
            level: DiagnosticLevel::Warning,
            category: "todo_item".to_string(),
            line: Some(42),
            message: "Unresolved TODO item".to_string(),
        };
        let yaml = serde_yaml::to_string(&diag).unwrap();
        let de: HealthDiagnostic = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(diag, de);
    }

    #[test]
    fn manuscript_health_report_defaults_and_methods() {
        let mut report = ManuscriptHealthReport::default();
        assert!(!report.has_errors());
        assert_eq!(report.warning_count(), 0);

        report.diagnostics.push(HealthDiagnostic {
            level: DiagnosticLevel::Info,
            category: "info".to_string(),
            line: None,
            message: "Word count ok".to_string(),
        });
        assert!(!report.has_errors());
        assert_eq!(report.warning_count(), 0);

        report.diagnostics.push(HealthDiagnostic {
            level: DiagnosticLevel::Warning,
            category: "todo".to_string(),
            line: Some(10),
            message: "Fix warning".to_string(),
        });
        assert!(!report.has_errors());
        assert_eq!(report.warning_count(), 1);

        report.diagnostics.push(HealthDiagnostic {
            level: DiagnosticLevel::Error,
            category: "missing_citation".to_string(),
            line: Some(20),
            message: "Missing key".to_string(),
        });
        assert!(report.has_errors());
        assert_eq!(report.warning_count(), 1);
    }

    #[test]
    fn manuscript_health_report_serde() {
        let report = ManuscriptHealthReport {
            diagnostics: vec![HealthDiagnostic {
                level: DiagnosticLevel::Error,
                category: "test".to_string(),
                line: Some(1),
                message: "msg".to_string(),
            }],
            word_count: 500,
            missing_citations_count: 1,
            unreferenced_labels_count: 2,
            todo_ideas_count: 3,
            total_bib_keys_count: 5,
            cited_bib_keys_count: 4,
        };
        assert_eq!(report.bib_citation_ratio(), (4, 5));
        assert_eq!(report.unmentioned_bib_keys_count(), 1);

        let yaml = serde_yaml::to_string(&report).unwrap();
        let de: ManuscriptHealthReport = serde_yaml::from_str(&yaml).unwrap();
        assert_eq!(report, de);
    }
}
