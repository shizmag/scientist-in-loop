//! Commit proposal construction (never auto-applied by production CLI).

use sil_core::SciAction;

/// A proposed (not applied) commit.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitProposal {
    /// Subject line.
    pub subject: String,
    /// Optional body paragraphs.
    pub body: Vec<String>,
    /// Sci-Action trailer.
    pub action: SciAction,
}

impl CommitProposal {
    /// Create a proposal with subject and Sci-Action.
    pub fn new(subject: impl Into<String>, action: SciAction) -> Self {
        Self {
            subject: subject.into(),
            body: Vec::new(),
            action,
        }
    }

    /// Add a body paragraph.
    pub fn with_body(mut self, paragraph: impl Into<String>) -> Self {
        self.body.push(paragraph.into());
        self
    }

    /// Full commit message including Sci-Action trailer.
    pub fn message(&self) -> String {
        let mut msg = self.subject.clone();
        if !self.body.is_empty() {
            msg.push_str("\n\n");
            msg.push_str(&self.body.join("\n\n"));
        }
        msg.push_str("\n\n");
        msg.push_str(&self.action.trailer_line());
        msg.push('\n');
        msg
    }

    /// Human-readable proposal block for the terminal.
    pub fn display(&self) -> String {
        format!(
            "Proposed commit (not applied):\n---\n{}---\nRun: git add -A && git commit -F - <<'EOF'\n{}EOF",
            self.message(),
            self.message()
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proposal_includes_trailer() {
        let p = CommitProposal::new("Initialize sil project", SciAction::Init)
            .with_body("Created workspace layout and database.");
        let msg = p.message();
        assert!(msg.contains("Initialize sil project"));
        assert!(msg.contains("Sci-Action: init"));
        assert_eq!(sil_core::extract_from_message(&msg), Some(SciAction::Init));
    }

    #[test]
    fn parse_pdf_trailer() {
        let p = CommitProposal::new("Parse source PDF", SciAction::ParsePdf);
        assert!(p.message().contains("Sci-Action: parse-pdf"));
    }

    #[test]
    fn all_action_trailers_in_proposals() {
        for a in [
            SciAction::Init,
            SciAction::ParsePdf,
            SciAction::UpdateStructure,
            SciAction::AddFigure,
            SciAction::AddData,
            SciAction::EditDraft,
            SciAction::PromoteToFinal,
            SciAction::FetchSource,
            SciAction::UpdateBibliography,
            SciAction::PromoteBibliography,
            SciAction::EstimatePaper,
            SciAction::GroundClaims,
        ] {
            let p = CommitProposal::new("subject", a);
            assert!(p.message().contains(&a.trailer_line()));
            assert!(p.display().contains("not applied"));
        }
    }

    #[test]
    fn multiple_body_paragraphs() {
        let p = CommitProposal::new("S", SciAction::AddData)
            .with_body("first")
            .with_body("second");
        let msg = p.message();
        assert!(msg.contains("first"));
        assert!(msg.contains("second"));
        assert!(msg.contains("Sci-Action: add-data"));
    }

    #[test]
    fn trailer_is_last_nonempty_block() {
        let p = CommitProposal::new("Subject line", SciAction::EditDraft).with_body("details");
        let msg = p.message();
        let trimmed = msg.trim_end();
        assert!(trimmed.ends_with("Sci-Action: edit-draft"), "{trimmed:?}");
    }

    #[test]
    fn empty_subject_still_has_trailer() {
        let p = CommitProposal::new("", SciAction::PromoteToFinal);
        assert!(p.message().contains("Sci-Action: promote-to-final"));
    }
}
