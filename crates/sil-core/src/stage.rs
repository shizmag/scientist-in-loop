//! Project lifecycle stage.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::ValidationError;

/// High-level project stage stored in config and structure.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Stage {
    /// Early drafting.
    #[default]
    Draft,
    /// Preparation / polishing before review.
    Prep,
    /// Under review.
    Review,
    /// Final / camera-ready.
    Final,
}

impl Stage {
    /// Canonical lowercase string form used in YAML and trailers.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Prep => "prep",
            Self::Review => "review",
            Self::Final => "final",
        }
    }
}

impl fmt::Display for Stage {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for Stage {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().as_str() {
            "draft" => Ok(Self::Draft),
            "prep" => Ok(Self::Prep),
            "review" => Ok(Self::Review),
            "final" => Ok(Self::Final),
            other => Err(ValidationError::InvalidStage(other.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_roundtrip() {
        for s in [Stage::Draft, Stage::Prep, Stage::Review, Stage::Final] {
            assert_eq!(Stage::from_str(s.as_str()).unwrap(), s);
            assert_eq!(s.to_string(), s.as_str());
        }
    }

    #[test]
    fn stage_invalid() {
        assert!(Stage::from_str("shipped").is_err());
    }

    #[test]
    fn stage_serde() {
        let yaml = "draft";
        let s: Stage = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(s, Stage::Draft);
        assert_eq!(
            serde_yaml::to_string(&Stage::Final).unwrap().trim(),
            "final"
        );
    }
}
