//! PDF parsing orchestration via Marker (Python helper).

#![deny(missing_docs)]

/// Batch PDF parsing and hydration.
pub mod batch;
mod error;
mod fetch;
mod interactive;
/// Journal digest resolution and CrossRef/arXiv API lookups.
pub mod journal_digest;
mod marker;
/// Extracting and cleaning reference entries from parsed text.
pub mod references;
mod validate;
/// CrossRef (xberg) metadata fetching.
pub mod xberg_metadata;
/// Incremental DOI checking and background orchestrator.
pub mod doi_checker;

pub use batch::{ParseResult, hydrate_source_document_metadata, parse_many, parse_one};
pub use doi_checker::{
    check_bib_dois_incremental, spawn_background_bib_doi_check, BibDoiItemReport,
    DoiCheckCategory, DoiCheckReport,
};
pub use error::ParseError;

pub use fetch::fetch_source_target;
pub use interactive::{
    SelectionEvent, SelectionOutcome, apply_selection_event, select_pdfs_interactive,
};
pub use journal_digest::{
    ReferenceBibResolution, SourceBibResolution, TitleLookupOutcome, fetch_journal_publications,
    fetch_journal_publications_native, fetch_work_by_arxiv_id, fetch_work_by_doi,
    lookup_doi_by_title, lookup_doi_by_title_detailed, resolve_official_bibtex_entry,
    resolve_official_bibtex_for_source, title_similarity,
};
pub use marker::{
    CliMarkerRunner, MarkerRunner, PythonMarkerRunner, StubMarkerRunner, discover_marker_runner,
};
pub use validate::{list_unparsed_pdfs, minimal_pdf_bytes, validate_for_parse, write_fixture_pdf};

#[cfg(test)]
mod tests {
    use super::*;
    use camino::{Utf8Path, Utf8PathBuf};
    use sil_core::NullUi;
    use sil_db::SilDb;

    #[test]
    fn reject_missing() {
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "hello".into(),
        };
        let err = parse_one(Utf8Path::new("/no/such.pdf"), &db, &runner, &ui).unwrap_err();
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn parse_with_stub() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("paper.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let path = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "transformer attention mechanism".into(),
        };
        let result = parse_one(&path, &db, &runner, &ui).unwrap();
        assert!(result.document.parsed);
        assert!(db.is_parsed(&result.document.id).unwrap());
        let err = parse_one(&path, &db, &runner, &ui).unwrap_err();
        assert!(err.to_string().contains("already parsed"));
    }

    #[test]
    fn verify_database_fields_populated_correctly() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("attention.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let path = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "# Attention Is All You Need\n\nAshish Vaswani, Noam Shazeer, Niki Parmar\n\nAbstract\nThe dominant sequence transduction models...\n\nReferences\n1. A. Vaswani et al. Attention Is All You Need. NeurIPS 2017.".into(),
        };

        let result = parse_one(&path, &db, &runner, &ui).unwrap();
        assert!(result.document.parsed);

        // Fetch from database and sample check fields
        let sources = db.list_sources().unwrap();
        let doc = sources
            .iter()
            .find(|s| s.id == result.document.id)
            .expect("Document must exist in DB");
        assert_eq!(doc.title.as_deref(), Some("Attention Is All You Need"));
        assert!(doc.authors.as_ref().unwrap().contains("Vaswani"));
        assert!(doc.references_text.is_some());

        // Check reference entries in database
        let refs = db.get_references_for_source(&result.document.id).unwrap();
        assert!(
            !refs.is_empty(),
            "Extracted references must be stored in database"
        );
        assert!(
            refs[0].raw_text.contains("Vaswani")
                || refs[0].title.as_deref().unwrap_or("").contains("Attention")
        );
    }

    #[test]
    fn reject_unsupported_format() {
        let dir = tempfile::tempdir().unwrap();
        let f = dir.path().join("notes.unsupported");
        std::fs::write(&f, "not a supported format").unwrap();
        let path = Utf8PathBuf::from_path_buf(f).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "x".into(),
        };
        let err = parse_one(&path, &db, &runner, &ui).unwrap_err();
        assert!(
            err.to_string()
                .to_lowercase()
                .contains("unsupported format")
        );
    }

    #[test]
    fn noninteractive_select_all() {
        let paths = vec![Utf8PathBuf::from("a.pdf"), Utf8PathBuf::from("b.pdf")];
        let ui = NullUi::new();
        let sel = select_pdfs_interactive(&paths, &ui).unwrap();
        assert_eq!(sel, vec![0, 1]);
    }

    #[test]
    fn select_empty_paths() {
        let ui = NullUi::new();
        let sel = select_pdfs_interactive(&[], &ui).unwrap();
        assert!(sel.is_empty());
    }

    #[test]
    fn list_unparsed_skips_parsed_and_unsupported_format() {
        let dir = tempfile::tempdir().unwrap();
        let sources = Utf8PathBuf::from_path_buf(dir.path().join("sources")).unwrap();
        std::fs::create_dir_all(sources.as_str()).unwrap();
        write_fixture_pdf(sources.join("keep.pdf").as_std_path()).unwrap();
        write_fixture_pdf(sources.join("done.pdf").as_std_path()).unwrap();
        std::fs::write(sources.join("data.unsupported").as_str(), "x").unwrap();

        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "done body".into(),
        };
        parse_one(&sources.join("done.pdf"), &db, &runner, &ui).unwrap();

        let unparsed = list_unparsed_pdfs(&sources, &db).unwrap();
        assert_eq!(unparsed.len(), 1);
        assert!(unparsed[0].as_str().ends_with("keep.pdf"));
    }

    #[test]
    fn parse_many_partial_failures() {
        let dir = tempfile::tempdir().unwrap();
        let good = dir.path().join("good.pdf");
        write_fixture_pdf(&good).unwrap();
        let good = Utf8PathBuf::from_path_buf(good).unwrap();
        let bad = Utf8PathBuf::from_path_buf(dir.path().join("missing.pdf")).unwrap();

        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "ok".into(),
        };
        let (ok, failed, errors) = parse_many(&[good, bad], &db, &runner, &ui);
        assert_eq!(ok, 1);
        assert_eq!(failed, 1);
        assert_eq!(errors.len(), 1);
    }

    #[test]
    fn validate_for_parse_already_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("p.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let path = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "c".into(),
        };
        parse_one(&path, &db, &runner, &ui).unwrap();
        let (status, _) = validate_for_parse(&path, &db).unwrap();
        assert_eq!(status, sil_core::DocumentStatus::AlreadyParsed);
    }

    #[test]
    fn minimal_pdf_has_magic() {
        let b = minimal_pdf_bytes();
        assert!(b.starts_with(b"%PDF"));
    }

    #[test]
    fn stub_runner_includes_filename() {
        let runner = StubMarkerRunner {
            content: "body".into(),
        };
        let text = runner.parse_pdf(Utf8Path::new("/tmp/paper.pdf")).unwrap();
        assert!(text.contains("paper.pdf"));
        assert!(text.contains("body"));
    }

    struct FailingMarker;

    impl MarkerRunner for FailingMarker {
        fn parse_pdf(&self, _pdf: &Utf8Path) -> Result<String, ParseError> {
            Err(ParseError::Marker(
                "exit 1: marker crashed\ngarbage on stdout".into(),
            ))
        }
    }

    #[test]
    fn marker_failure_surfaces_clean_error() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("x.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let path = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let err = parse_one(&path, &db, &FailingMarker, &ui).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Marker") || msg.contains("marker") || msg.contains("exit 1"));
        assert!(!db.is_parsed(&sil_core::SourceId::new("x.pdf")).unwrap());
    }

    #[test]
    fn list_unparsed_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        let sources = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let list = list_unparsed_pdfs(&sources, &db).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_unparsed_missing_dir() {
        let db = SilDb::open_in_memory().unwrap();
        let list = list_unparsed_pdfs(Utf8Path::new("/no/such/sources"), &db).unwrap();
        assert!(list.is_empty());
    }

    #[test]
    fn list_unparsed_mix_parsed_and_unparsed() {
        let dir = tempfile::tempdir().unwrap();
        let sources = Utf8PathBuf::from_path_buf(dir.path().join("sources")).unwrap();
        std::fs::create_dir_all(sources.as_str()).unwrap();
        write_fixture_pdf(sources.join("old.pdf").as_std_path()).unwrap();
        write_fixture_pdf(sources.join("new.pdf").as_std_path()).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "old".into(),
        };
        parse_one(&sources.join("old.pdf"), &db, &runner, &ui).unwrap();
        let unparsed = list_unparsed_pdfs(&sources, &db).unwrap();
        assert_eq!(unparsed.len(), 1);
        assert!(unparsed[0].as_str().ends_with("new.pdf"));
    }

    #[test]
    fn selection_cancel_returns_empty() {
        let mut selected = vec![true, true];
        let mut cursor = 0;
        let out = apply_selection_event(SelectionEvent::Cancel, &mut selected, &mut cursor);
        assert_eq!(out, SelectionOutcome::Cancelled);
    }

    #[test]
    fn selection_confirm_with_toggles() {
        let mut selected = vec![true, true, true];
        let mut cursor = 1;
        apply_selection_event(SelectionEvent::Toggle, &mut selected, &mut cursor);
        apply_selection_event(SelectionEvent::None, &mut selected, &mut cursor);
        apply_selection_event(SelectionEvent::Toggle, &mut selected, &mut cursor);
        let out = apply_selection_event(SelectionEvent::Confirm, &mut selected, &mut cursor);
        assert_eq!(out, SelectionOutcome::Confirmed(vec![1]));
    }

    #[test]
    fn selection_cursor_bounds() {
        let mut selected = vec![true, false];
        let mut cursor = 0;
        apply_selection_event(SelectionEvent::Up, &mut selected, &mut cursor);
        assert_eq!(cursor, 0);
        apply_selection_event(SelectionEvent::Down, &mut selected, &mut cursor);
        assert_eq!(cursor, 1);
        apply_selection_event(SelectionEvent::Down, &mut selected, &mut cursor);
        assert_eq!(cursor, 1);
    }

    #[test]
    fn corrupt_pdf_extension_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("broken.pdf");
        std::fs::write(&path, b"not really a pdf at all").unwrap();
        let path = Utf8PathBuf::from_path_buf(path).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "x".into(),
        };
        let err = parse_one(&path, &db, &runner, &ui).unwrap_err();
        let msg = err.to_string().to_lowercase();
        assert!(
            msg.contains("not a pdf") || msg.contains("corrupt"),
            "{msg}"
        );
    }

    #[test]
    fn parse_directory_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let path = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "x".into(),
        };
        let err = parse_one(&path, &db, &runner, &ui).unwrap_err();
        assert!(
            err.to_string().to_lowercase().contains("not a pdf")
                || err.to_string().contains("not found")
                || err.to_string().contains("corrupt")
        );
    }

    #[test]
    fn uppercase_pdf_extension_listed() {
        let dir = tempfile::tempdir().unwrap();
        let sources = Utf8PathBuf::from_path_buf(dir.path().join("sources")).unwrap();
        std::fs::create_dir_all(sources.as_str()).unwrap();
        write_fixture_pdf(sources.join("X.PDF").as_std_path()).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let list = list_unparsed_pdfs(&sources, &db).unwrap();
        assert_eq!(list.len(), 1);
    }

    #[test]
    fn marker_empty_content_still_marks_parsed() {
        let dir = tempfile::tempdir().unwrap();
        let pdf = dir.path().join("e.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let path = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: String::new(),
        };
        let r = parse_one(&path, &db, &runner, &ui).unwrap();
        assert!(r.document.parsed);
        assert!(db.is_parsed(&r.document.id).unwrap());
    }

    #[test]
    fn selection_all_then_confirm() {
        let mut selected = vec![false, false];
        let mut cursor = 0;
        apply_selection_event(SelectionEvent::All, &mut selected, &mut cursor);
        let out = apply_selection_event(SelectionEvent::Confirm, &mut selected, &mut cursor);
        assert_eq!(out, SelectionOutcome::Confirmed(vec![0, 1]));
    }

    #[test]
    fn selection_none_then_confirm_empty() {
        let mut selected = vec![true, true];
        let mut cursor = 0;
        apply_selection_event(SelectionEvent::None, &mut selected, &mut cursor);
        let out = apply_selection_event(SelectionEvent::Confirm, &mut selected, &mut cursor);
        assert_eq!(out, SelectionOutcome::Confirmed(vec![]));
    }

    #[test]
    fn parse_many_all_fail() {
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "x".into(),
        };
        let missing = Utf8PathBuf::from("/no/a.pdf");
        let missing2 = Utf8PathBuf::from("/no/b.pdf");
        let (ok, failed, errors) = parse_many(&[missing, missing2], &db, &runner, &ui);
        assert_eq!(ok, 0);
        assert_eq!(failed, 2);
        assert_eq!(errors.len(), 2);
    }

    #[test]
    fn python_marker_nonzero_exit() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("fail.py");
        std::fs::write(
            &script,
            "import sys\nprint('err', file=sys.stderr)\nsys.exit(2)\n",
        )
        .unwrap();
        let script = Utf8PathBuf::from_path_buf(script).unwrap();
        let runner = PythonMarkerRunner {
            script,
            python: "python3".into(),
        };
        let pdf = dir.path().join("t.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let pdf = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let err = runner.parse_pdf(&pdf).unwrap_err();
        let msg = err.to_string();
        assert!(
            msg.contains("exit") || msg.contains("Marker") || msg.contains("2"),
            "{msg}"
        );
    }

    #[test]
    fn parse_markdown_natively() {
        let dir = tempfile::tempdir().unwrap();
        let md_file = dir.path().join("notes.md");
        std::fs::write(
            &md_file,
            "# Markdown Title\nThis is native markdown content.",
        )
        .unwrap();
        let path = Utf8PathBuf::from_path_buf(md_file).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "should not be called".into(),
        };
        let result = parse_one(&path, &db, &runner, &ui).unwrap();
        assert!(result.document.parsed);
        assert_eq!(result.document.kind, sil_core::SourceKind::Markdown);
        assert_eq!(result.document.title.as_deref(), Some("Markdown Title"));
        assert!(db.is_parsed(&result.document.id).unwrap());
    }

    #[test]
    fn parse_text_natively() {
        let dir = tempfile::tempdir().unwrap();
        let txt_file = dir.path().join("abstract.txt");
        std::fs::write(&txt_file, "Plain text content without headings.").unwrap();
        let path = Utf8PathBuf::from_path_buf(txt_file).unwrap();
        let db = SilDb::open_in_memory().unwrap();
        let ui = NullUi::new();
        let runner = StubMarkerRunner {
            content: "should not be called".into(),
        };
        let result = parse_one(&path, &db, &runner, &ui).unwrap();
        assert!(result.document.parsed);
        assert_eq!(result.document.kind, sil_core::SourceKind::Text);
        assert_eq!(result.document.title.as_deref(), Some("abstract"));
        assert!(db.is_parsed(&result.document.id).unwrap());
    }

    #[test]
    fn fetch_source_target_mock_script() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("download_mock.py");
        std::fs::write(
            &script,
            "import sys\nprint('Downloaded to sources/test.pdf')\n",
        )
        .unwrap();
        let dest = Utf8PathBuf::from_path_buf(dir.path().join("sources")).unwrap();

        unsafe {
            std::env::set_var("SIL_DOWNLOAD_SCRIPT", &script);
        }
        let res = fetch_source_target("10.1234/test", &dest).unwrap();
        unsafe {
            std::env::remove_var("SIL_DOWNLOAD_SCRIPT");
        }
        assert_eq!(res.as_str(), "Downloaded to sources/test.pdf");
    }

    #[test]
    fn cli_marker_runner_mock_test() {
        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("mock_marker.sh");
        std::fs::write(
            &script,
            "#!/bin/sh\nOUT_DIR=\"$3\"\nmkdir -p \"$OUT_DIR/doc\"\necho '# Mock Extracted Content' > \"$OUT_DIR/doc/doc.md\"\n",
        )
        .unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = std::fs::metadata(&script).unwrap().permissions();
            perms.set_mode(0o755);
            std::fs::set_permissions(&script, perms).unwrap();
        }
        let script = Utf8PathBuf::from_path_buf(script).unwrap();
        let runner = CliMarkerRunner::new(script);
        let pdf = dir.path().join("t.pdf");
        write_fixture_pdf(&pdf).unwrap();
        let pdf = Utf8PathBuf::from_path_buf(pdf).unwrap();
        let content = runner.parse_pdf(&pdf).unwrap();
        assert!(content.contains("# Mock Extracted Content"));
    }
}
