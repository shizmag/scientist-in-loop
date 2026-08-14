use crate::app::{App, CommandId, InputMode};
use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sil_core::ProjectPaths;
use tempfile::tempdir;

#[test]
fn review_changes_builds_read_only_fixture_view() {
    let dir = tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    std::fs::create_dir_all(ProjectPaths::new(&root).sil_dir().as_std_path()).unwrap();
    sil_git::init_repo(&root).unwrap();
    std::fs::write(root.join("paper_draft.tex"), "draft").unwrap();
    std::fs::write(root.join("references.bib"), "bib").unwrap();

    let mut app = App::new(Some(root));
    app.dispatch(CommandId::ReviewChanges);

    assert_eq!(app.input_mode, InputMode::ProposalDiff);
    assert!(
        app.proposal_diff_content
            .as_deref()
            .unwrap()
            .contains("Status:")
    );
    assert!(
        app.proposal_text
            .as_deref()
            .unwrap()
            .contains("not applied")
    );
}

#[test]
fn discard_without_undo_journal_reports_noop() {
    let dir = tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    std::fs::create_dir_all(ProjectPaths::new(&root).sil_dir().as_std_path()).unwrap();
    let mut app = App::new(Some(root));
    app.input_mode = InputMode::ProposalDiff;

    app.handle_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::empty()));

    assert!(app.status_message.contains("No TUI undo journal"));
}
