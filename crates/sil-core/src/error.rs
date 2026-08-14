//! Error types for library crates (thiserror).

use thiserror::Error;

/// Validation failures for domain values.
#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum ValidationError {
    /// Unknown project stage string.
    #[error("invalid stage '{0}'; expected draft | prep | review | final")]
    InvalidStage(String),
    /// Unknown LaTeX engine string.
    #[error(
        "invalid latex engine '{0}'; expected tectonic | latexmk | pdflatex | xelatex | lualatex"
    )]
    InvalidLatexEngine(String),
    /// Unknown Sci-Action string.
    #[error("invalid Sci-Action '{0}'")]
    InvalidSciAction(String),
    /// Unknown section completion string.
    #[error("invalid section completion '{0}'; expected empty | outline | draft | polished")]
    InvalidCompletion(String),
    /// I/O failure during validation.
    #[error("I/O error at {path}: {message}")]
    Io {
        /// Path involved.
        path: String,
        /// Underlying error message.
        message: String,
    },
    /// Generic validation message.
    #[error("{0}")]
    Message(String),
}

/// Configuration load/parse errors.
#[derive(Debug, Error)]
pub enum ConfigError {
    /// File missing.
    #[error("config not found at {0}")]
    NotFound(String),
    /// YAML parse failure.
    #[error("invalid config YAML at {path}: {source}")]
    Parse {
        /// Config path.
        path: String,
        /// Serde/YAML error.
        #[source]
        source: serde_yaml::Error,
    },
    /// I/O failure.
    #[error("failed to read config at {path}: {source}")]
    Io {
        /// Config path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Semantic validation failure.
    #[error("invalid config: {0}")]
    Validation(#[from] ValidationError),
}

/// structure.yaml load/parse errors.
#[derive(Debug, Error)]
pub enum StructureError {
    /// File missing.
    #[error("structure not found at {0}")]
    NotFound(String),
    /// YAML parse failure.
    #[error("invalid structure YAML at {path}: {source}")]
    Parse {
        /// Structure path.
        path: String,
        /// Serde/YAML error.
        #[source]
        source: serde_yaml::Error,
    },
    /// I/O failure.
    #[error("failed to read structure at {path}: {source}")]
    Io {
        /// Structure path.
        path: String,
        /// Underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// Semantic validation failure.
    #[error("invalid structure: {0}")]
    Validation(#[from] ValidationError),
}

/// Top-level library error used across sil crates.
#[derive(Debug, Error)]
pub enum SilError {
    /// Not inside a sil project.
    #[error("not a sil project (missing .sil/config.yaml); run `sil init` first")]
    NotAProject,
    /// Config error.
    #[error(transparent)]
    Config(#[from] ConfigError),
    /// Structure error.
    #[error(transparent)]
    Structure(#[from] StructureError),
    /// Validation error.
    #[error(transparent)]
    Validation(#[from] ValidationError),
    /// Database error (stringified to keep sil-core free of rusqlite).
    #[error("database error: {0}")]
    Database(String),
    /// Git error.
    #[error("git error: {0}")]
    Git(String),
    /// Parse / Marker error.
    #[error("parse error: {0}")]
    Parse(String),
    /// LaTeX build error.
    #[error("build error: {0}")]
    Build(String),
    /// Source fetch error.
    #[error("fetch error: {0}")]
    Fetch(String),
    /// Generic I/O.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Other message.
    #[error("{0}")]
    Message(String),
}

/// User-facing, categorized error representation.
///
/// Designed to provide clean, actionable error messages on the status bar or CLI
/// without exposing raw `Debug` dumps or internal stack traces.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct UserError {
    /// Stable machine-readable error code (e.g. `"crossref.rate_limited"`).
    pub code: &'static str,
    /// Human-friendly short error title for status bars or summaries.
    pub title: String,
    /// Actionable remediation or explanation for the user.
    pub hint: String,
    /// Optional command or action identifier to retry or resolve.
    pub retry: Option<&'static str>,
}

impl std::fmt::Display for UserError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.title)
    }
}

impl std::error::Error for UserError {}

impl UserError {
    /// Create a new [`UserError`].
    pub fn new(
        code: &'static str,
        title: impl Into<String>,
        hint: impl Into<String>,
        retry: Option<&'static str>,
    ) -> Self {
        Self {
            code,
            title: title.into(),
            hint: hint.into(),
            retry,
        }
    }

    /// Classify an arbitrary error string into a structured [`UserError`].
    pub fn classify(err_str: &str) -> Self {
        let lower = err_str.to_lowercase();

        if lower.contains("429") || lower.contains("rate limit") || lower.contains("rate_limit") {
            Self::new(
                "crossref.rate_limited",
                "Literature service is busy",
                "Retry in a few seconds. Palette: Retry last job",
                Some("retry-last-job"),
            )
        } else if lower.contains("offline")
            || lower.contains("connection refused")
            || lower.contains("dns")
            || lower.contains("network")
            || lower.contains("failed to lookup")
        {
            Self::new(
                "network.offline",
                "Network connection failed",
                "Check internet connectivity and try again",
                Some("retry-last-job"),
            )
        } else if lower.contains("engine")
            || lower.contains("latex engine")
            || lower.contains("tectonic")
            || lower.contains("pdflatex")
            || lower.contains("no latex")
        {
            Self::new(
                "latex.engine_missing",
                "LaTeX engine not found",
                "Install tectonic (`brew install tectonic`) or configure another engine",
                None,
            )
        } else if lower.contains("marker-pdf")
            || lower.contains("marker missing")
            || lower.contains("marker")
        {
            Self::new(
                "parse.marker_missing",
                "PDF parser (Marker) missing",
                "Install marker or use text/markdown sources",
                None,
            )
        } else if lower.contains("database is locked")
            || lower.contains("sqlite")
            || lower.contains("busy")
        {
            Self::new(
                "sqlite.busy",
                "Database is busy",
                "Another process is accessing the database. Please wait and retry",
                Some("retry-last-job"),
            )
        } else if lower.contains("not a sil project")
            || lower.contains("not inside a sil project")
            || lower.contains("missing .sil")
            || lower.contains("config not found")
            || lower.contains("structure not found")
            || lower.contains("project not found")
            || lower.contains("no active project")
            || lower == "not found"
        {
            Self::new(
                "project.not_found",
                "Not inside a sil project",
                "Run `sil init` to initialize a project or open an existing project",
                None,
            )
        } else if lower.contains("workspace.lock")
            || lower.contains("workspace lock")
            || lower.contains("lock")
        {
            Self::new(
                "lock.held",
                "Workspace lock is held",
                "Another sil instance or agent may be active",
                None,
            )
        } else if lower.contains("failed to parse")
            || lower.contains("syntax error")
            || lower.contains("parse")
        {
            Self::new(
                "parse.failed",
                "Failed to parse source",
                "Check source file syntax and formatting",
                None,
            )
        } else {
            let sanitized_title = sanitize_error_title(err_str);
            Self::new(
                "internal.error",
                sanitized_title,
                "Check logs for technical details",
                None,
            )
        }
    }

    /// Construct a [`UserError`] by classifying a message string.
    pub fn from_message(msg: &str) -> Self {
        Self::classify(msg)
    }
}

impl From<&str> for UserError {
    fn from(msg: &str) -> Self {
        Self::classify(msg)
    }
}

impl From<String> for UserError {
    fn from(msg: String) -> Self {
        Self::classify(&msg)
    }
}

impl From<&SilError> for UserError {
    fn from(err: &SilError) -> Self {
        Self::classify(&err.to_string())
    }
}

impl From<SilError> for UserError {
    fn from(err: SilError) -> Self {
        Self::classify(&err.to_string())
    }
}

fn sanitize_error_title(err_str: &str) -> String {
    let trimmed = err_str.trim();
    if trimmed.is_empty() {
        return "Operation failed".to_string();
    }
    let first_line = trimmed.lines().next().unwrap_or("Operation failed").trim();
    if first_line.is_empty() {
        return "Operation failed".to_string();
    }
    let clean = if let Some(stripped) = first_line.strip_prefix("Error: ") {
        stripped
    } else if let Some(stripped) = first_line.strip_prefix("error: ") {
        stripped
    } else if let Some(stripped) = first_line.strip_prefix("failed: ") {
        stripped
    } else {
        first_line
    };
    let clean = clean.trim();
    if clean.is_empty() {
        return "Operation failed".to_string();
    }
    if clean.chars().count() > 80 {
        let truncated: String = clean.chars().take(79).collect();
        format!("{truncated}…")
    } else {
        clean.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validation_messages_are_actionable() {
        let s = ValidationError::InvalidStage("x".into()).to_string();
        assert!(s.contains("draft"));
        let e = ValidationError::InvalidLatexEngine("z".into()).to_string();
        assert!(e.contains("tectonic"));
        let c = ValidationError::InvalidCompletion("done".into()).to_string();
        assert!(c.contains("outline") || c.contains("polished"));
    }

    #[test]
    fn sil_error_not_a_project_mentions_init() {
        let msg = SilError::NotAProject.to_string();
        assert!(msg.contains("sil init") || msg.contains("init"));
        assert!(msg.contains(".sil") || msg.contains("project"));
    }

    #[test]
    fn sil_error_wrappers_display() {
        assert!(
            SilError::Database("locked".into())
                .to_string()
                .contains("database")
        );
        assert!(SilError::Git("boom".into()).to_string().contains("git"));
        assert!(SilError::Parse("x".into()).to_string().contains("parse"));
        assert!(SilError::Build("y".into()).to_string().contains("build"));
        assert!(SilError::Fetch("z".into()).to_string().contains("fetch"));
    }

    #[test]
    fn config_not_found_display() {
        let e = ConfigError::NotFound("/tmp/x".into());
        assert!(e.to_string().contains("not found"));
    }

    #[test]
    fn test_user_error_rate_limit_mapping() {
        let e1 = UserError::classify("HTTP status 429 Too Many Requests");
        assert_eq!(e1.code, "crossref.rate_limited");
        assert_eq!(e1.title, "Literature service is busy");
        assert_eq!(e1.hint, "Retry in a few seconds. Palette: Retry last job");
        assert_eq!(e1.retry, Some("retry-last-job"));

        let e2 = UserError::classify("Rate limit exceeded for Crossref API");
        assert_eq!(e2.code, "crossref.rate_limited");

        let e3 = UserError::from_message("crossref: rate_limit hit");
        assert_eq!(e3.code, "crossref.rate_limited");
    }

    #[test]
    fn test_user_error_network_mapping() {
        let e = UserError::classify("connection refused to server");
        assert_eq!(e.code, "network.offline");
        assert_eq!(e.title, "Network connection failed");
        assert_eq!(e.retry, Some("retry-last-job"));
    }

    #[test]
    fn test_user_error_latex_engine_mapping() {
        let e = UserError::classify("tectonic executable not found");
        assert_eq!(e.code, "latex.engine_missing");
        assert_eq!(e.title, "LaTeX engine not found");
        assert_eq!(e.retry, None);
    }

    #[test]
    fn test_user_error_marker_missing_mapping() {
        let e = UserError::classify("marker missing in environment");
        assert_eq!(e.code, "parse.marker_missing");
        assert_eq!(e.title, "PDF parser (Marker) missing");
        assert_eq!(e.retry, None);
    }

    #[test]
    fn test_user_error_sqlite_busy_mapping() {
        let e = UserError::classify("sqlite: database is locked");
        assert_eq!(e.code, "sqlite.busy");
        assert_eq!(e.title, "Database is busy");
        assert_eq!(e.retry, Some("retry-last-job"));
    }

    #[test]
    fn test_user_error_project_not_found_mapping() {
        let e1 = UserError::classify("not a sil project (missing .sil/config.yaml)");
        assert_eq!(e1.code, "project.not_found");
        assert_eq!(e1.title, "Not inside a sil project");
        assert_eq!(
            e1.hint,
            "Run `sil init` to initialize a project or open an existing project"
        );
        assert_eq!(e1.retry, None);

        let e2 = UserError::classify("missing .sil folder");
        assert_eq!(e2.code, "project.not_found");

        let e3 = UserError::classify("config not found at /path/to/project");
        assert_eq!(e3.code, "project.not_found");
    }

    #[test]
    fn test_user_error_lock_held_mapping() {
        let e = UserError::classify("workspace.lock exists");
        assert_eq!(e.code, "lock.held");
        assert_eq!(e.title, "Workspace lock is held");
        assert_eq!(e.retry, None);
    }

    #[test]
    fn test_user_error_parse_failed_mapping() {
        let e = UserError::classify("syntax error: failed to parse YAML");
        assert_eq!(e.code, "parse.failed");
        assert_eq!(e.title, "Failed to parse source");
        assert_eq!(e.retry, None);
    }

    #[test]
    fn test_user_error_display_equals_title() {
        let e = UserError::classify("HTTP status 429 Too Many Requests");
        assert_eq!(e.to_string(), e.title);
        assert_eq!(e.to_string(), "Literature service is busy");

        let custom = UserError::new("custom.code", "Short summary", "Do something", None);
        assert_eq!(custom.to_string(), "Short summary");
        assert_eq!(format!("{custom}"), custom.title);
    }

    #[test]
    fn test_user_error_fallback_sanitization() {
        let fallback =
            UserError::classify("Error: some unknown low-level failure\nstack backtrace:\n0: foo");
        assert_eq!(fallback.code, "internal.error");
        assert_eq!(fallback.title, "some unknown low-level failure");
        assert_eq!(fallback.hint, "Check logs for technical details");
        assert_eq!(fallback.retry, None);

        let empty = UserError::classify("");
        assert_eq!(empty.code, "internal.error");
        assert_eq!(empty.title, "Operation failed");
    }
}
