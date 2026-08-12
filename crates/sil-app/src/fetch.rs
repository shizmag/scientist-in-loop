//! Source document fetching use-case ([`fetch_source`]).

use camino::{Utf8Path, Utf8PathBuf};
use sil_core::SourceDocument;
use sil_db::SilDb;
use sil_git::{CommitProposal, SciAction, proposal_for_action};

use crate::bib::{UpsertBib, UpsertBibResult, upsert_bib};
use crate::context::AppContext;
use crate::error::AppError;

/// Request payload for [`fetch_source`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FetchSource {
    /// Target DOI, arXiv ID, or URL to download.
    pub target: String,
    /// Whether to attempt parsing the downloaded source document.
    pub parse: bool,
}

/// Summary of a parsed source document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseSummary {
    /// Filename of the parsed document.
    pub filename: String,
    /// Title extracted during parsing, if any.
    pub title: Option<String>,
    /// Unique source document ID.
    pub source_id: String,
    /// Number of references extracted from the document.
    pub reference_count: usize,
}

/// Result returned by [`fetch_source`].
#[derive(Debug, Clone)]
pub struct FetchSourceResult {
    /// On-disk path to the downloaded source file.
    pub downloaded_path: Utf8PathBuf,
    /// Summary of document parsing if parsing was requested and succeeded.
    pub parsed: Option<ParseSummary>,
    /// Error message if parsing was requested but failed or runner was missing.
    pub parse_error: Option<String>,
    /// Result of upserting official BibTeX into `references.bib`, if resolved.
    pub bib: Option<UpsertBibResult>,
    /// Git commit proposal for downloading the source.
    pub fetch_proposal: CommitProposal,
    /// Git commit proposal for parsing the source document, if parsed.
    pub parse_proposal: Option<CommitProposal>,
}

/// Download a source document by DOI, arXiv ID, or URL, optionally parse it, and upsert official BibTeX metadata.
///
/// Download failure is a hard error returning `Err(AppError::Parse(...))`. Missing official bib metadata is non-fatal.
pub fn fetch_source(ctx: &AppContext, req: FetchSource) -> Result<FetchSourceResult, AppError> {
    let sources_dir = ctx.paths.sources(&ctx.config);
    let saved = sil_parse::fetch_source_target(&req.target, &sources_dir)?;

    let saved_str = saved.as_str().trim();
    let p = Utf8Path::new(saved_str);
    let downloaded_path = if p.is_absolute() {
        p.to_path_buf()
    } else if sources_dir
        .join(p.file_name().unwrap_or(saved_str))
        .exists()
    {
        sources_dir.join(p.file_name().unwrap_or(saved_str))
    } else {
        ctx.root.join(p)
    };

    let mut parsed: Option<ParseSummary> = None;
    let mut parse_error: Option<String> = None;
    let mut parse_proposal: Option<CommitProposal> = None;
    let mut parse_res: Option<sil_parse::ParseResult> = None;

    if req.parse && downloaded_path.exists() {
        match sil_parse::discover_marker_runner() {
            Err(e) => {
                parse_error = Some(e.to_string());
            }
            Ok(runner) => match SilDb::open(&ctx.paths.db()) {
                Err(e) => {
                    parse_error = Some(e.to_string());
                }
                Ok(db) => {
                    let null_ui = sil_core::NullUi::new();
                    match sil_parse::parse_one(&downloaded_path, &db, runner.as_ref(), &null_ui) {
                        Err(e) => {
                            parse_error = Some(e.to_string());
                        }
                        Ok(res) => {
                            parsed = Some(ParseSummary {
                                filename: res.document.filename.clone(),
                                title: res.document.title.clone(),
                                source_id: res.document.id.as_str().to_string(),
                                reference_count: res.reference_count,
                            });
                            parse_proposal = Some(proposal_for_action(
                                SciAction::ParsePdf,
                                Some(&format!("Parse PDF: {}", res.document.filename)),
                                Some(&format!("Parsed source document {}", res.document.filename)),
                            ));
                            parse_res = Some(res);
                        }
                    }
                }
            },
        }
    }

    let mut official_bib: Option<String> = None;

    if let Some(doi) = sil_regex::extract_doi(&req.target)
        && let Ok(Some(bib)) = sil_parse::journal_digest::fetch_bibtex_by_doi(&doi)
    {
        official_bib = Some(bib);
    } else if let Some(arxiv) = sil_regex::extract_arxiv_id(&req.target)
        && let Ok(Some(bib)) = sil_parse::journal_digest::fetch_bibtex_by_arxiv_id(&arxiv)
    {
        official_bib = Some(bib);
    }

    if official_bib.is_none() {
        let doc = if let Some(ref res) = parse_res {
            res.document.clone()
        } else {
            SourceDocument::new(downloaded_path.clone())
        };
        if let sil_parse::SourceBibResolution::Resolved(bib) =
            sil_parse::journal_digest::resolve_official_bibtex_for_source(&doc)
        {
            official_bib = Some(bib);
        }
    }

    let bib = if let Some(bib_str) = official_bib {
        upsert_bib(
            ctx,
            UpsertBib {
                entry: bib_str,
                draft: false,
            },
        )
        .ok()
    } else {
        None
    };

    let fetch_proposal = proposal_for_action(
        SciAction::FetchSource,
        Some(&format!("Fetch source: {}", req.target)),
        Some(&format!("Saved to {saved}")),
    );

    Ok(FetchSourceResult {
        downloaded_path,
        parsed,
        parse_error,
        bib,
        fetch_proposal,
        parse_proposal,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::Mutex;
    use tempfile::TempDir;

    static ENV_LOCK: Mutex<()> = Mutex::new(());

    fn setup_temp_project() -> (TempDir, AppContext) {
        let dir = tempfile::tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        fs::create_dir_all(root.join(".sil")).unwrap();
        fs::write(root.join(".sil/config.yaml"), "project:\n  title: Test\n").unwrap();
        let ctx = AppContext::from_root(&root).unwrap();
        (dir, ctx)
    }

    struct EnvGuard<'a> {
        _lock: std::sync::MutexGuard<'a, ()>,
        var_name: &'static str,
    }

    impl<'a> EnvGuard<'a> {
        fn set(lock: &'a Mutex<()>, var_name: &'static str, val: &std::path::Path) -> Self {
            let guard = lock.lock().unwrap();
            unsafe {
                std::env::set_var(var_name, val);
            }
            Self {
                _lock: guard,
                var_name,
            }
        }
    }

    impl Drop for EnvGuard<'_> {
        fn drop(&mut self) {
            unsafe {
                std::env::remove_var(self.var_name);
            }
        }
    }

    #[test]
    fn test_fetch_source_download_failure_is_hard_error() {
        let (_dir, ctx) = setup_temp_project();
        let script = _dir.path().join("fail_download.py");
        fs::write(
            &script,
            "import sys\nprint('failed download', file=sys.stderr)\nsys.exit(1)\n",
        )
        .unwrap();

        let _guard = EnvGuard::set(&ENV_LOCK, "SIL_DOWNLOAD_SCRIPT", &script);

        let err = fetch_source(
            &ctx,
            FetchSource {
                target: "10.1234/fail".into(),
                parse: true,
            },
        )
        .unwrap_err();

        assert!(matches!(err, AppError::Parse(_)));
        let bib_path = ctx.paths.join(sil_core::paths::rel::REFERENCES);
        assert!(!bib_path.exists());
    }

    #[test]
    fn test_fetch_source_success_no_parse() {
        let (_dir, ctx) = setup_temp_project();
        let script = _dir.path().join("mock_download.py");
        fs::write(
            &script,
            r#"import sys, os
dest = sys.argv[2]
os.makedirs(dest, exist_ok=True)
pdf_path = os.path.join(dest, "test.pdf")
with open(pdf_path, "wb") as f:
    f.write(b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n")
print("sources/test.pdf")
"#,
        )
        .unwrap();

        let _guard = EnvGuard::set(&ENV_LOCK, "SIL_DOWNLOAD_SCRIPT", &script);

        let res = fetch_source(
            &ctx,
            FetchSource {
                target: "https://example.com/test.pdf".into(),
                parse: false,
            },
        )
        .unwrap();

        assert!(res.downloaded_path.exists());
        assert!(res.parsed.is_none());
        assert!(res.parse_error.is_none());
        assert!(res.parse_proposal.is_none());
        assert!(
            res.fetch_proposal
                .message()
                .contains("Sci-Action: fetch-source")
        );
    }

    #[test]
    fn test_fetch_source_success_with_stub_parse() {
        let (_dir, ctx) = setup_temp_project();
        let script = _dir.path().join("mock_download.py");
        fs::write(
            &script,
            r#"import sys, os
dest = sys.argv[2]
os.makedirs(dest, exist_ok=True)
pdf_path = os.path.join(dest, "test.pdf")
with open(pdf_path, "wb") as f:
    f.write(b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n")
print("sources/test.pdf")
"#,
        )
        .unwrap();

        let _download_guard = EnvGuard::set(&ENV_LOCK, "SIL_DOWNLOAD_SCRIPT", &script);

        unsafe {
            std::env::set_var("SIL_MARKER_STUB", "# Test Paper Title\n\nAuthor Name");
        }

        let res = fetch_source(
            &ctx,
            FetchSource {
                target: "https://example.com/test.pdf".into(),
                parse: true,
            },
        )
        .unwrap();

        unsafe {
            std::env::remove_var("SIL_MARKER_STUB");
        }

        assert!(res.downloaded_path.exists());
        assert!(res.parsed.is_some());
        let summary = res.parsed.as_ref().unwrap();
        assert_eq!(summary.filename, "test.pdf");
        assert_eq!(summary.title.as_deref(), Some("Test Paper Title"));
        assert!(res.parse_proposal.is_some());
        assert!(res.parse_error.is_none());
    }

    #[test]
    fn test_fetch_source_parse_true_runner_missing() {
        let (_dir, ctx) = setup_temp_project();
        let script = _dir.path().join("mock_download.py");
        fs::write(
            &script,
            r#"import sys, os
dest = sys.argv[2]
os.makedirs(dest, exist_ok=True)
pdf_path = os.path.join(dest, "test.pdf")
with open(pdf_path, "wb") as f:
    f.write(b"%PDF-1.4\n1 0 obj\n<<>>\nendobj\ntrailer\n<<>>\n%%EOF\n")
print("sources/test.pdf")
"#,
        )
        .unwrap();

        let _download_guard = EnvGuard::set(&ENV_LOCK, "SIL_DOWNLOAD_SCRIPT", &script);

        unsafe {
            std::env::remove_var("SIL_MARKER_STUB");
            std::env::remove_var("SIL_MARKER_BIN");
            std::env::remove_var("SIL_PARSE_SCRIPT");
        }

        let res = fetch_source(
            &ctx,
            FetchSource {
                target: "https://example.com/test.pdf".into(),
                parse: true,
            },
        )
        .unwrap();

        assert!(res.downloaded_path.exists());
        assert!(res.parsed.is_none());
        assert!(res.parse_error.is_some());
        assert!(res.parse_proposal.is_none());
    }
}
