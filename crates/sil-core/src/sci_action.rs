//! Sci-Action trailers for git commit proposals.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use crate::error::ValidationError;

/// Scientific workflow action recorded in git commit trailers.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "kebab-case")]
pub enum SciAction {
    /// Project initialization.
    Init,
    /// Project templates / managed files upgraded (`sil init --update`).
    Update,
    /// PDF parsed into SQLite/FTS5.
    ParsePdf,
    /// structure.yaml updated.
    UpdateStructure,
    /// Figure added.
    AddFigure,
    /// Data added.
    AddData,
    /// Draft manuscript edited.
    EditDraft,
    /// Content promoted to paper.tex.
    PromoteToFinal,
    /// Source PDF fetched.
    FetchSource,
}

impl SciAction {
    /// Trailer value (kebab-case).
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Init => "init",
            Self::Update => "update",
            Self::ParsePdf => "parse-pdf",
            Self::UpdateStructure => "update-structure",
            Self::AddFigure => "add-figure",
            Self::AddData => "add-data",
            Self::EditDraft => "edit-draft",
            Self::PromoteToFinal => "promote-to-final",
            Self::FetchSource => "fetch-source",
        }
    }

    /// Full trailer line: `Sci-Action: <value>`.
    pub fn trailer_line(self) -> String {
        format!("Sci-Action: {}", self.as_str())
    }

    /// Trailer key constant.
    pub const TRAILER_KEY: &'static str = "Sci-Action";
}

impl fmt::Display for SciAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl FromStr for SciAction {
    type Err = ValidationError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        // Accept bare value or full trailer line.
        let value = s
            .strip_prefix("Sci-Action:")
            .or_else(|| s.strip_prefix("sci-action:"))
            .map(str::trim)
            .unwrap_or(s);
        match value.to_ascii_lowercase().as_str() {
            "init" => Ok(Self::Init),
            "update" | "init-update" => Ok(Self::Update),
            "parse-pdf" => Ok(Self::ParsePdf),
            "update-structure" => Ok(Self::UpdateStructure),
            "add-figure" => Ok(Self::AddFigure),
            "add-data" => Ok(Self::AddData),
            "edit-draft" => Ok(Self::EditDraft),
            "promote-to-final" => Ok(Self::PromoteToFinal),
            "fetch-source" => Ok(Self::FetchSource),
            other => Err(ValidationError::InvalidSciAction(other.to_string())),
        }
    }
}

/// Extract a Sci-Action from free-form commit message body if present.
pub fn extract_from_message(message: &str) -> Option<SciAction> {
    for line in message.lines() {
        let line = line.trim();
        if line.to_ascii_lowercase().starts_with("sci-action:") {
            return SciAction::from_str(line).ok();
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn trailer_formatting() {
        assert_eq!(SciAction::Init.trailer_line(), "Sci-Action: init");
        assert_eq!(SciAction::ParsePdf.trailer_line(), "Sci-Action: parse-pdf");
        assert_eq!(
            SciAction::FetchSource.trailer_line(),
            "Sci-Action: fetch-source"
        );
    }

    #[test]
    fn parse_bare_and_trailer() {
        assert_eq!(
            SciAction::from_str("parse-pdf").unwrap(),
            SciAction::ParsePdf
        );
        assert_eq!(
            SciAction::from_str("Sci-Action: fetch-source").unwrap(),
            SciAction::FetchSource
        );
    }

    #[test]
    fn extract_from_commit_message() {
        let msg = "Initialize project\n\nSci-Action: init\n";
        assert_eq!(extract_from_message(msg), Some(SciAction::Init));
        assert_eq!(extract_from_message("no trailer"), None);
    }

    #[test]
    fn all_actions_roundtrip() {
        let actions = [
            SciAction::Init,
            SciAction::Update,
            SciAction::ParsePdf,
            SciAction::UpdateStructure,
            SciAction::AddFigure,
            SciAction::AddData,
            SciAction::EditDraft,
            SciAction::PromoteToFinal,
            SciAction::FetchSource,
        ];
        for a in actions {
            assert_eq!(SciAction::from_str(a.as_str()).unwrap(), a);
            assert_eq!(
                SciAction::from_str(&a.trailer_line()).unwrap(),
                a
            );
            assert!(a.trailer_line().starts_with("Sci-Action: "));
            assert_eq!(a.to_string(), a.as_str());
        }
    }

    #[test]
    fn trailer_key_constant() {
        assert_eq!(SciAction::TRAILER_KEY, "Sci-Action");
    }

    #[test]
    fn invalid_action() {
        assert!(SciAction::from_str("deploy").is_err());
        assert!(SciAction::from_str("Sci-Action: bogus").is_err());
    }

    #[test]
    fn extract_case_insensitive_key() {
        let msg = "subject\n\nsci-action: parse-pdf\n";
        assert_eq!(extract_from_message(msg), Some(SciAction::ParsePdf));
    }
}
