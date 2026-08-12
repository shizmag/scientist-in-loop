//! Bibliography management use-cases (`upsert_bib` and `promote_bib`).

use std::fs;

use camino::Utf8PathBuf;
use sil_core::bib::{
    UpsertOptions, extract_bib_entry_info, is_same_paper, is_tui_added_bib_block,
    mark_tui_added_bib_entry, parse_bib_blocks, unmark_tui_added_bib_entry,
    upsert_bib_entry_with_options,
};
use sil_git::{CommitProposal, SciAction, proposal_for_action};

use crate::context::AppContext;
use crate::error::AppError;

/// Request payload for [`upsert_bib`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UpsertBib {
    /// Raw BibTeX entry string to insert or update.
    pub entry: String,
    /// Whether this entry should be marked as draft (`% [sil: tui-added]`).
    pub draft: bool,
}

/// Result returned by [`upsert_bib`].
#[derive(Debug, Clone)]
pub struct UpsertBibResult {
    /// Resolved cite key of the upserted BibTeX entry.
    pub cite_key: String,
    /// Whether an existing entry in `references.bib` was replaced.
    pub replaced: bool,
    /// Path to the updated `references.bib` file.
    pub path: Utf8PathBuf,
    /// Whether the entry was written with a draft marker.
    pub draft: bool,
    /// Git commit proposal for the bibliography update.
    pub proposal: CommitProposal,
}

/// Request payload for [`promote_bib`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromoteBib {
    /// Cite key or paper identity (DOI / arXiv ID / title) to promote.
    pub target: String,
}

/// Result returned by [`promote_bib`].
#[derive(Debug, Clone)]
pub struct PromoteBibResult {
    /// Resolved cite key of the promoted entry.
    pub cite_key: String,
    /// Whether the entry previously had the draft marker (`% [sil: tui-added]`).
    pub had_marker: bool,
    /// Path to the updated `references.bib` file.
    pub path: Utf8PathBuf,
    /// Git commit proposal for the bibliography promotion.
    pub proposal: CommitProposal,
}

/// Upsert a BibTeX entry into the project's `references.bib`.
///
/// Validates that `req.entry` is non-empty and contains `@`. Preserves existing cite keys
/// when replacing an entry for the same paper (`preserve_cite_key` is always true).
/// Atomically updates disk and generates a [`CommitProposal`].
pub fn upsert_bib(ctx: &AppContext, req: UpsertBib) -> Result<UpsertBibResult, AppError> {
    if req.entry.trim().is_empty() {
        return Err(AppError::InvalidBib("entry must not be empty".to_string()));
    }
    if !req.entry.contains('@') {
        return Err(AppError::InvalidBib(
            "entry is not valid BibTeX (missing @type{key, ...})".to_string(),
        ));
    }

    let bib_path = ctx.paths.join(sil_core::paths::rel::REFERENCES);
    let current = fs::read_to_string(bib_path.as_str()).unwrap_or_default();

    let entry_for_upsert = if req.draft {
        mark_tui_added_bib_entry(&req.entry)
    } else {
        req.entry.to_string()
    };

    let (updated, replaced) = upsert_bib_entry_with_options(
        &current,
        &entry_for_upsert,
        UpsertOptions {
            preserve_cite_key: true,
        },
    );

    let new_info = extract_bib_entry_info(&entry_for_upsert);
    let cite_key = parse_bib_blocks(&updated)
        .into_iter()
        .find(|block| is_same_paper(&extract_bib_entry_info(block), &new_info))
        .and_then(|block| extract_bib_entry_info(&block).cite_key)
        .or_else(|| new_info.cite_key.clone())
        .unwrap_or_else(|| "unknown".to_string());

    sil_core::write_atomic_str(&bib_path, &updated).map_err(|source| AppError::Io {
        path: bib_path.to_string(),
        source,
    })?;

    let proposal = proposal_for_action(
        SciAction::UpdateBibliography,
        Some(&format!("Update bibliography: {cite_key}")),
        Some(&format!(
            "Upserted BibTeX entry '{cite_key}' into {} (draft={}, preserve_cite_key=true, replaced={replaced})",
            sil_core::paths::rel::REFERENCES,
            req.draft,
        )),
    );

    Ok(UpsertBibResult {
        cite_key,
        replaced,
        path: bib_path,
        draft: req.draft,
        proposal,
    })
}

/// Promote a draft BibTeX entry in `references.bib` by removing its draft marker.
///
/// Matches target against entry cite key (case-insensitive) or paper identity (`is_same_paper`).
/// Returns an error if `references.bib` is missing or if no matching entry is found.
pub fn promote_bib(ctx: &AppContext, req: PromoteBib) -> Result<PromoteBibResult, AppError> {
    let target = req.target.trim();
    if target.is_empty() {
        return Err(AppError::InvalidBib(
            "target cite key or identity must not be empty".to_string(),
        ));
    }

    let bib_path = ctx.paths.join(sil_core::paths::rel::REFERENCES);
    if !bib_path.is_file() {
        return Err(AppError::NotFound(format!(
            "references.bib not found at {bib_path}"
        )));
    }

    let current = fs::read_to_string(bib_path.as_str()).map_err(|source| AppError::Io {
        path: bib_path.to_string(),
        source,
    })?;

    let target_info = sil_core::BibEntryInfo {
        cite_key: Some(target.to_string()),
        title: Some(target.to_string()),
        doi: Some(target.to_string()),
        arxiv_id: Some(target.to_string()),
        is_incomplete: false,
    };

    let mut blocks = parse_bib_blocks(&current);
    let mut promoted_key: Option<String> = None;
    let mut had_marker = false;

    for block in &mut blocks {
        let block_info = extract_bib_entry_info(block);
        let key_match = block_info
            .cite_key
            .as_deref()
            .unwrap_or("")
            .eq_ignore_ascii_case(target);
        if is_same_paper(&block_info, &target_info) || key_match {
            let key = block_info.cite_key.as_deref().unwrap_or(target).to_string();
            had_marker = is_tui_added_bib_block(block);
            *block = unmark_tui_added_bib_entry(block);
            promoted_key = Some(key);
            break;
        }
    }

    let Some(cite_key) = promoted_key else {
        return Err(AppError::NotFound(format!(
            "No entry matching '{target}' found in {bib_path} to promote"
        )));
    };

    let updated = if blocks.is_empty() {
        String::new()
    } else {
        blocks.join("\n\n") + "\n"
    };

    sil_core::write_atomic_str(&bib_path, &updated).map_err(|source| AppError::Io {
        path: bib_path.to_string(),
        source,
    })?;

    let proposal = proposal_for_action(
        SciAction::PromoteBibliography,
        Some(&format!("Promote bibliography entry: {cite_key}")),
        Some(&format!(
            "Removed % [sil: tui-added] from '{cite_key}' in {}",
            sil_core::paths::rel::REFERENCES
        )),
    );

    Ok(PromoteBibResult {
        cite_key,
        had_marker,
        path: bib_path,
        proposal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn setup_temp_project() -> (TempDir, AppContext) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join(".sil")).unwrap();
        fs::write(root.join(".sil/config.yaml"), "project:\n  title: Test\n").unwrap();
        let ctx = AppContext::from_root(&root).unwrap();
        (dir, ctx)
    }

    #[test]
    fn test_upsert_new_entry() {
        let (_dir, ctx) = setup_temp_project();
        let entry = r#"@article{vaswani2017attention,
  title={Attention is all you need},
  author={Vaswani, Ashish and others},
  journal={Advances in neural information processing systems},
  volume={30},
  year={2017}
}"#;

        let res = upsert_bib(
            &ctx,
            UpsertBib {
                entry: entry.to_string(),
                draft: false,
            },
        )
        .unwrap();

        assert_eq!(res.cite_key, "vaswani2017attention");
        assert!(!res.replaced);
        assert!(!res.draft);
        assert!(res.path.exists());

        let content = fs::read_to_string(res.path.as_str()).unwrap();
        assert!(content.contains("@article{vaswani2017attention"));
        assert!(
            res.proposal
                .message()
                .contains("Sci-Action: update-bibliography")
        );
    }

    #[test]
    fn test_upsert_existing_paper_preserves_cite_key() {
        let (_dir, ctx) = setup_temp_project();
        let initial_entry = r#"@article{vaswani2017,
  title={Attention is all you need},
  author={Vaswani, Ashish},
  doi={10.48550/arXiv.1706.03762}
}"#;

        let res1 = upsert_bib(
            &ctx,
            UpsertBib {
                entry: initial_entry.to_string(),
                draft: false,
            },
        )
        .unwrap();
        assert_eq!(res1.cite_key, "vaswani2017");
        assert!(!res1.replaced);

        let updated_entry_different_key = r#"@article{newkey2026,
  title={Attention is All You Need},
  author={Vaswani, Ashish and Niki Parmar},
  doi={10.48550/arXiv.1706.03762}
}"#;

        let res2 = upsert_bib(
            &ctx,
            UpsertBib {
                entry: updated_entry_different_key.to_string(),
                draft: false,
            },
        )
        .unwrap();

        assert_eq!(res2.cite_key, "vaswani2017");
        assert!(res2.replaced);

        let content = fs::read_to_string(res2.path.as_str()).unwrap();
        assert!(content.contains("vaswani2017"));
        assert!(!content.contains("newkey2026"));
    }

    #[test]
    fn test_upsert_draft_writes_marker() {
        let (_dir, ctx) = setup_temp_project();
        let entry = r#"@article{draft2024,
  title={Draft Paper},
  author={Author}
}"#;

        let res = upsert_bib(
            &ctx,
            UpsertBib {
                entry: entry.to_string(),
                draft: true,
            },
        )
        .unwrap();

        assert!(res.draft);
        let content = fs::read_to_string(res.path.as_str()).unwrap();
        assert!(content.contains("% [sil: tui-added]"));
    }

    #[test]
    fn test_promote_strips_marker() {
        let (_dir, ctx) = setup_temp_project();
        let entry = r#"@article{draft2024,
  title={Draft Paper},
  author={Author}
}"#;

        upsert_bib(
            &ctx,
            UpsertBib {
                entry: entry.to_string(),
                draft: true,
            },
        )
        .unwrap();

        let res = promote_bib(
            &ctx,
            PromoteBib {
                target: "draft2024".to_string(),
            },
        )
        .unwrap();

        assert_eq!(res.cite_key, "draft2024");
        assert!(res.had_marker);
        assert!(
            res.proposal
                .message()
                .contains("Sci-Action: promote-bibliography")
        );

        let content = fs::read_to_string(res.path.as_str()).unwrap();
        assert!(!content.contains("% [sil: tui-added]"));
    }

    #[test]
    fn test_promote_unknown_target_errors() {
        let (_dir, ctx) = setup_temp_project();
        // Create an empty references.bib first so file exists
        let bib_path = ctx.paths.join(sil_core::paths::rel::REFERENCES);
        fs::write(bib_path.as_str(), "").unwrap();

        let err = promote_bib(
            &ctx,
            PromoteBib {
                target: "nonexistent_key".to_string(),
            },
        )
        .unwrap_err();

        match err {
            AppError::NotFound(msg) => assert!(msg.contains("nonexistent_key")),
            other => panic!("expected AppError::NotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_promote_missing_file_errors() {
        let (_dir, ctx) = setup_temp_project();
        let err = promote_bib(
            &ctx,
            PromoteBib {
                target: "somekey".to_string(),
            },
        )
        .unwrap_err();

        match err {
            AppError::NotFound(msg) => assert!(msg.contains("references.bib not found")),
            other => panic!("expected AppError::NotFound, got {other:?}"),
        }
    }

    #[test]
    fn test_upsert_empty_or_non_bibtex_errors() {
        let (_dir, ctx) = setup_temp_project();

        let err1 = upsert_bib(
            &ctx,
            UpsertBib {
                entry: "   ".to_string(),
                draft: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err1, AppError::InvalidBib(_)));

        let err2 = upsert_bib(
            &ctx,
            UpsertBib {
                entry: "title = Attention is all you need".to_string(),
                draft: false,
            },
        )
        .unwrap_err();
        assert!(matches!(err2, AppError::InvalidBib(_)));
    }

    #[test]
    fn test_from_cwd_without_project_errors() {
        let dir = tempfile::tempdir().unwrap();
        let _guard = std::env::set_current_dir(dir.path());
        let err = AppContext::from_cwd();
        assert!(matches!(err, Err(AppError::NotInProject)));
    }
}
