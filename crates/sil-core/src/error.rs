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
}
