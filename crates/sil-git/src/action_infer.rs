//! Infer Sci-Action from dirty paths or explicit names.

use sil_core::SciAction;
use sil_core::paths::rel;

use crate::propose::CommitProposal;
use crate::status::GitStatus;

/// Infer the primary Sci-Action from changed paths (first matching rule wins).
pub fn infer_action_from_paths(paths: &[String]) -> Option<SciAction> {
    let mut has_draft = false;
    let mut has_structure = false;
    let mut has_final = false;
    let mut has_source = false;
    let mut has_figure = false;
    let mut has_data = false;

    for p in paths {
        let p = p.trim_start_matches("./");
        if p == rel::PAPER_DRAFT || p.ends_with("/paper_draft.tex") {
            has_draft = true;
        } else if p == rel::STRUCTURE || p.ends_with("structure.yaml") {
            has_structure = true;
        } else if p == rel::PAPER_FINAL || p.ends_with("/paper.tex") {
            has_final = true;
        } else if p.starts_with("sources/") || p == "sources" {
            has_source = true;
        } else if p.starts_with("figures/") {
            has_figure = true;
        } else if p.starts_with("data/") {
            has_data = true;
        }
    }

    // Prefer explicit paper workflow actions
    if has_final {
        return Some(SciAction::PromoteToFinal);
    }
    if has_structure {
        return Some(SciAction::UpdateStructure);
    }
    if has_draft {
        return Some(SciAction::EditDraft);
    }
    if has_source {
        return Some(SciAction::FetchSource);
    }
    if has_figure {
        return Some(SciAction::AddFigure);
    }
    if has_data {
        return Some(SciAction::AddData);
    }
    None
}

/// Collect relative paths from a git status snapshot.
pub fn dirty_paths(status: &GitStatus) -> Vec<String> {
    status
        .entries
        .iter()
        .filter_map(|e| {
            let path = e.get(3..).unwrap_or(e.as_str()).trim();
            let path = path.rsplit_once(" -> ").map(|(_, n)| n).unwrap_or(path);
            if path.is_empty() {
                None
            } else {
                Some(path.to_string())
            }
        })
        .collect()
}

/// Build a commit proposal for an explicit action and optional body.
pub fn proposal_for_action(
    action: SciAction,
    subject: Option<&str>,
    body: Option<&str>,
) -> CommitProposal {
    let default_subject = match action {
        SciAction::Init => "Initialize sil project",
        SciAction::Update => "Update sil project templates",
        SciAction::ParsePdf => "Parse source PDF",
        SciAction::UpdateStructure => "Update paper structure",
        SciAction::AddFigure => "Add figure",
        SciAction::AddData => "Add data",
        SciAction::EditDraft => "Edit paper draft",
        SciAction::PromoteToFinal => "Promote draft to final manuscript",
        SciAction::FetchSource => "Fetch source PDF",
    };
    let mut p = CommitProposal::new(subject.unwrap_or(default_subject), action);
    if let Some(b) = body
        && !b.trim().is_empty()
    {
        p = p.with_body(b);
    }
    p
}

/// Propose a commit from git status (path-aware) or an explicit Sci-Action.
pub fn propose_from_status(
    status: &GitStatus,
    explicit: Option<SciAction>,
    subject: Option<&str>,
    body: Option<&str>,
) -> Result<CommitProposal, String> {
    let paths = dirty_paths(status);
    let action = match explicit {
        Some(a) => a,
        None => infer_action_from_paths(&paths).ok_or_else(|| {
            if paths.is_empty() {
                "working tree clean; pass --action to propose a Sci-Action anyway".into()
            } else {
                format!(
                    "could not infer Sci-Action from dirty paths: {}; pass --action",
                    paths.join(", ")
                )
            }
        })?,
    };
    let mut p = proposal_for_action(action, subject, body);
    if !paths.is_empty() && body.is_none() {
        p = p.with_body(format!("Changed paths:\n{}", paths.iter().map(|x| format!("- {x}")).collect::<Vec<_>>().join("\n")));
    }
    Ok(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_edit_draft() {
        assert_eq!(
            infer_action_from_paths(&["paper_draft.tex".into()]),
            Some(SciAction::EditDraft)
        );
    }

    #[test]
    fn infer_structure_and_promote() {
        assert_eq!(
            infer_action_from_paths(&[".sil/structure.yaml".into()]),
            Some(SciAction::UpdateStructure)
        );
        assert_eq!(
            infer_action_from_paths(&["paper.tex".into(), "paper_draft.tex".into()]),
            Some(SciAction::PromoteToFinal)
        );
    }

    #[test]
    fn explicit_proposal_has_trailer() {
        let p = proposal_for_action(SciAction::EditDraft, None, Some("notes"));
        assert!(p.message().contains("Sci-Action: edit-draft"));
        assert!(p.message().contains("notes"));
    }

    #[test]
    fn propose_from_clean_needs_action() {
        let st = GitStatus {
            is_repo: true,
            entries: vec![],
            clean: true,
        };
        assert!(propose_from_status(&st, None, None, None).is_err());
        let p = propose_from_status(&st, Some(SciAction::EditDraft), None, None).unwrap();
        assert!(p.message().contains("Sci-Action: edit-draft"));
    }

    #[test]
    fn propose_from_dirty_infers() {
        let st = GitStatus {
            is_repo: true,
            entries: vec![" M paper_draft.tex".into()],
            clean: false,
        };
        let p = propose_from_status(&st, None, None, None).unwrap();
        assert!(p.message().contains("Sci-Action: edit-draft"));
        assert!(p.message().contains("paper_draft.tex"));
    }
}
