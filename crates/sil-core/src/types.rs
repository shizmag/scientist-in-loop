//! Shared domain enums and project handle.

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::config::Config;
use crate::error::ValidationError;
use crate::stage::Stage;

/// LaTeX compilation engine selected in config.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum LatexEngine {
    /// Tectonic (self-contained).
    #[default]
    Tectonic,
    /// latexmk wrapper.
    Latexmk,
    /// Classic pdflatex.
    Pdflatex,
    /// XeLaTeX.
    Xelatex,
    /// LuaLaTeX.
    Lualatex,
}

impl LatexEngine {
    /// Command name invoked for this engine.
    pub fn command(self) -> &'static str {
        match self {
            Self::Tectonic => "tectonic",
            Self::Latexmk => "latexmk",
            Self::Pdflatex => "pdflatex",
            Self::Xelatex => "xelatex",
            Self::Lualatex => "lualatex",
        }
    }

    /// Canonical string form.
    pub fn as_str(self) -> &'static str {
        self.command()
    }
}

impl fmt::Display for LatexEngine {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for LatexEngine {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "tectonic" => Ok(Self::Tectonic),
            "latexmk" => Ok(Self::Latexmk),
            "pdflatex" => Ok(Self::Pdflatex),
            "xelatex" => Ok(Self::Xelatex),
            "lualatex" => Ok(Self::Lualatex),
            other => Err(ValidationError::InvalidLatexEngine(other.to_string())),
        }
    }
}

/// Which paper artifact is being addressed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum PaperKind {
    /// Working draft (`paper_draft.tex`).
    #[default]
    Draft,
    /// Cleaned final manuscript (`paper.tex`).
    Final,
}

impl PaperKind {
    /// Default filename for this paper kind.
    pub fn default_filename(self) -> &'static str {
        match self {
            Self::Draft => "paper_draft.tex",
            Self::Final => "paper.tex",
        }
    }

    /// Canonical string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Final => "final",
        }
    }
}

impl fmt::Display for PaperKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Loaded project root with config and stage.
#[derive(Debug, Clone)]
pub struct SilProject {
    /// Absolute (or normalized) project root.
    pub root: Utf8PathBuf,
    /// Parsed project configuration.
    pub config: Config,
    /// Current stage (mirrors config.project.stage).
    pub stage: Stage,
}

impl SilProject {
    /// Construct a project handle from root and config.
    pub fn new(root: Utf8PathBuf, config: Config) -> Self {
        let stage = config.project.stage;
        Self {
            root,
            config,
            stage,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn latex_engine_roundtrip() {
        for e in [
            LatexEngine::Tectonic,
            LatexEngine::Latexmk,
            LatexEngine::Pdflatex,
            LatexEngine::Xelatex,
            LatexEngine::Lualatex,
        ] {
            assert_eq!(LatexEngine::from_str(e.as_str()).unwrap(), e);
        }
    }

    #[test]
    fn paper_kind_filenames() {
        assert_eq!(PaperKind::Draft.default_filename(), "paper_draft.tex");
        assert_eq!(PaperKind::Final.default_filename(), "paper.tex");
        assert_eq!(PaperKind::Draft.as_str(), "draft");
        assert_eq!(PaperKind::Final.to_string(), "final");
    }

    #[test]
    fn latex_engine_commands() {
        assert_eq!(LatexEngine::Tectonic.command(), "tectonic");
        assert_eq!(LatexEngine::Latexmk.command(), "latexmk");
        assert_eq!(LatexEngine::Pdflatex.command(), "pdflatex");
        assert_eq!(LatexEngine::Xelatex.command(), "xelatex");
        assert_eq!(LatexEngine::Lualatex.command(), "lualatex");
    }

    #[test]
    fn latex_engine_invalid() {
        assert!(LatexEngine::from_str("context").is_err());
    }

    #[test]
    fn sil_project_mirrors_stage() {
        let mut cfg = Config::default();
        cfg.project.stage = Stage::Review;
        let proj = SilProject::new(Utf8PathBuf::from("/tmp/p"), cfg);
        assert_eq!(proj.stage, Stage::Review);
        assert_eq!(proj.root.as_str(), "/tmp/p");
    }
}
