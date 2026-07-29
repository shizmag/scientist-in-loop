//! Formal paper structure (`structure.yaml`).

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use camino::Utf8Path;

use crate::error::{StructureError, ValidationError};
use crate::stage::Stage;

/// Section completion level.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum SectionCompletion {
    /// Not started.
    #[default]
    Empty,
    /// Outline only.
    Outline,
    /// Draft prose present.
    Draft,
    /// Polished text.
    Polished,
}

impl SectionCompletion {
    /// Canonical string form.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Empty => "empty",
            Self::Outline => "outline",
            Self::Draft => "draft",
            Self::Polished => "polished",
        }
    }
}

impl fmt::Display for SectionCompletion {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SectionCompletion {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "empty" => Ok(Self::Empty),
            "outline" => Ok(Self::Outline),
            "draft" => Ok(Self::Draft),
            "polished" => Ok(Self::Polished),
            other => Err(ValidationError::InvalidCompletion(other.to_string())),
        }
    }
}

/// One section node in the syntactic tree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Section {
    /// Stable section id.
    pub id: String,
    /// Display title.
    pub title: String,
    /// Heading level (1 = top-level section).
    pub level: u32,
    /// Completion status.
    #[serde(default)]
    pub completion: SectionCompletion,
    /// Primary claim (concise).
    #[serde(default)]
    pub main_claim: String,
    /// Secondary bullet points.
    #[serde(default)]
    pub secondary_points: Vec<String>,
    /// Required content checklist.
    #[serde(default)]
    pub required_content: Vec<String>,
}

/// Full paper structure document.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Structure {
    /// Paper title.
    #[serde(default)]
    pub title: String,
    /// Overall status (mirrors stage vocabulary).
    #[serde(default)]
    pub status: Stage,
    /// Ordered section tree (flat list with levels).
    #[serde(default)]
    pub sections: Vec<Section>,
}

impl Default for Structure {
    fn default() -> Self {
        Self {
            title: String::new(),
            status: Stage::Draft,
            sections: Vec::new(),
        }
    }
}

impl Structure {
    /// Parse from YAML.
    pub fn from_yaml(yaml: &str) -> Result<Self, StructureError> {
        let s: Structure = serde_yaml::from_str(yaml).map_err(|source| StructureError::Parse {
            path: "<memory>".to_string(),
            source,
        })?;
        s.validate()?;
        Ok(s)
    }

    /// Load from file.
    pub fn load(path: &Utf8Path) -> Result<Self, StructureError> {
        if !path.exists() {
            return Err(StructureError::NotFound(path.to_string()));
        }
        let text = std::fs::read_to_string(path).map_err(|source| StructureError::Io {
            path: path.to_string(),
            source,
        })?;
        let s: Structure = serde_yaml::from_str(&text).map_err(|source| StructureError::Parse {
            path: path.to_string(),
            source,
        })?;
        s.validate()?;
        Ok(s)
    }

    /// Serialize to YAML.
    pub fn to_yaml(&self) -> Result<String, StructureError> {
        serde_yaml::to_string(self).map_err(|source| StructureError::Parse {
            path: "<memory>".to_string(),
            source,
        })
    }

    /// Validate invariants.
    pub fn validate(&self) -> Result<(), StructureError> {
        let mut ids = std::collections::HashSet::new();
        for sec in &self.sections {
            if sec.id.is_empty() {
                return Err(StructureError::Validation(ValidationError::Message(
                    "section id must not be empty".into(),
                )));
            }
            if !ids.insert(sec.id.clone()) {
                return Err(StructureError::Validation(ValidationError::Message(
                    format!("duplicate section id '{}'", sec.id),
                )));
            }
            if sec.level == 0 {
                return Err(StructureError::Validation(ValidationError::Message(
                    format!("section '{}' level must be >= 1", sec.id),
                )));
            }
        }
        Ok(())
    }

    /// Summary counts by completion level.
    pub fn completion_summary(&self) -> CompletionSummary {
        let mut summary = CompletionSummary::default();
        for sec in &self.sections {
            match sec.completion {
                SectionCompletion::Empty => summary.empty += 1,
                SectionCompletion::Outline => summary.outline += 1,
                SectionCompletion::Draft => summary.draft += 1,
                SectionCompletion::Polished => summary.polished += 1,
            }
        }
        summary.total = self.sections.len();
        summary
    }
}

/// Aggregate section completion counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct CompletionSummary {
    /// Total sections.
    pub total: usize,
    /// Empty count.
    pub empty: usize,
    /// Outline count.
    pub outline: usize,
    /// Draft count.
    pub draft: usize,
    /// Polished count.
    pub polished: usize,
}

impl fmt::Display for CompletionSummary {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{} sections (empty={}, outline={}, draft={}, polished={})",
            self.total, self.empty, self.outline, self.draft, self.polished
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"
title: "Attention Is All You Need – Replication Study"
status: draft
sections:
  - id: intro
    title: Introduction
    level: 1
    completion: outline
    main_claim: "The Transformer architecture outperforms RNN-based models."
    secondary_points:
      - "Self-attention reduces sequential computation."
    required_content:
      - "Clear problem statement"
  - id: related
    title: Related Work
    level: 1
    completion: empty
    main_claim: ""
    secondary_points: []
    required_content:
      - "RNN / LSTM sequence models"
"#;

    #[test]
    fn parse_structure() {
        let s = Structure::from_yaml(SAMPLE).unwrap();
        assert_eq!(s.sections.len(), 2);
        assert_eq!(s.sections[0].completion, SectionCompletion::Outline);
        let sum = s.completion_summary();
        assert_eq!(sum.outline, 1);
        assert_eq!(sum.empty, 1);
    }

    #[test]
    fn reject_duplicate_ids() {
        let bad = SAMPLE.replace("related", "intro");
        assert!(Structure::from_yaml(&bad).is_err());
    }

    #[test]
    fn completion_from_str() {
        assert_eq!(
            SectionCompletion::from_str("polished").unwrap(),
            SectionCompletion::Polished
        );
        assert!(SectionCompletion::from_str("done").is_err());
    }

    #[test]
    fn all_completions_roundtrip() {
        for c in [
            SectionCompletion::Empty,
            SectionCompletion::Outline,
            SectionCompletion::Draft,
            SectionCompletion::Polished,
        ] {
            assert_eq!(SectionCompletion::from_str(c.as_str()).unwrap(), c);
            assert_eq!(c.to_string(), c.as_str());
        }
    }

    #[test]
    fn reject_empty_section_id() {
        let yaml = r#"
title: T
status: draft
sections:
  - id: ""
    title: X
    level: 1
"#;
        assert!(Structure::from_yaml(yaml).is_err());
    }

    #[test]
    fn reject_zero_level() {
        let yaml = r#"
title: T
status: draft
sections:
  - id: intro
    title: Introduction
    level: 0
"#;
        assert!(Structure::from_yaml(yaml).is_err());
    }

    #[test]
    fn completion_summary_display() {
        let s = Structure::from_yaml(SAMPLE).unwrap();
        let text = s.completion_summary().to_string();
        assert!(text.contains("2 sections"));
        assert!(text.contains("outline=1"));
        assert!(text.contains("empty=1"));
    }

    #[test]
    fn load_structure_from_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = camino::Utf8PathBuf::from_path_buf(dir.path().join("structure.yaml")).unwrap();
        std::fs::write(path.as_str(), SAMPLE).unwrap();
        let s = Structure::load(&path).unwrap();
        assert_eq!(s.sections.len(), 2);
    }

    #[test]
    fn default_structure_empty() {
        let s = Structure::default();
        assert!(s.sections.is_empty());
        assert_eq!(s.completion_summary().total, 0);
    }

    #[test]
    fn structure_yaml_roundtrip() {
        let s = Structure::from_yaml(SAMPLE).unwrap();
        let yaml = s.to_yaml().unwrap();
        let again = Structure::from_yaml(&yaml).unwrap();
        assert_eq!(s, again);
    }

    #[test]
    fn invalid_completion_value() {
        let yaml = SAMPLE.replace("completion: outline", "completion: done");
        let err = Structure::from_yaml(&yaml).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("completion") || msg.contains("invalid") || msg.contains("yaml"),
            "{msg}"
        );
    }

    #[test]
    fn missing_structure_file() {
        let err = Structure::load(camino::Utf8Path::new("/no/such/structure.yaml")).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn invalid_yaml_syntax_structure() {
        let err = Structure::from_yaml("sections: [").unwrap_err();
        assert!(err.to_string().to_lowercase().contains("invalid") || err.to_string().contains("YAML") || err.to_string().contains("yaml"));
    }
}
