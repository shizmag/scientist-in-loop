use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn grounding_modal_is_read_only_for_draft() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.paper_draft_content = "\\section{Methods}\nDraft claim".to_string();
    app.paper_sections = sil_latex::split_tex_sections(&app.paper_draft_content);
    let before = app.paper_draft_content.clone();

    app.grounding_hits = vec![GroundingHit {
        title: "Fixture source".to_string(),
        score: 0.5,
        source_id: "fixture.md".to_string(),
    }];
    app.input_mode = InputMode::GroundingModal;
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    assert_eq!(app.paper_draft_content, before);
    assert_eq!(app.active_tab, ActiveTab::Sources);
}

#[test]
fn grounding_empty_results_close_without_panic() {
    let mut app = App::new(None);
    app.input_mode = InputMode::GroundingModal;
    app.grounding_hits.clear();

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::GroundingModal);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
}

#[test]
fn grounding_command_uses_empty_db_without_network() {
    let temp = tempfile::tempdir().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(temp.path().to_path_buf()).unwrap();
    std::fs::create_dir_all(root.join(".sil").as_std_path()).unwrap();
    std::fs::write(root.join("paper_draft.tex").as_std_path(), "Draft claim").unwrap();

    let mut app = App::new(Some(root));
    app.active_tab = ActiveTab::PaperDraft;
    app.handle_key(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::empty()));

    assert_eq!(app.input_mode, InputMode::GroundingModal);
    assert!(app.grounding_hits.is_empty());
    assert!(app.status_message.contains("No grounding sources"));
}
