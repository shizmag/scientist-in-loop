use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tempfile::tempdir;

#[test]
fn test_bib_delete_and_undo_restores_exact_references_bib() {
    let dir = tempdir().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let bib_path = root.join("references.bib");

    let initial_bib = "@article{vaswani2017,\n  author = {Vaswani, Ashish},\n  title = {Attention is All You Need},\n  year = {2017}\n}\n\n@article{devlin2018,\n  author = {Devlin, Jacob},\n  title = {BERT},\n  year = {2018}\n}";
    std::fs::write(bib_path.as_std_path(), initial_bib).unwrap();

    let mut app = App::new(Some(root.clone()));
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::LeftBib;
    app.load_project_references_bib();

    assert_eq!(app.bib_file_entries.len(), 2);
    app.selected_bib_index = 0;

    // Delete the first entry
    app.delete_selected_bib_entry();
    assert_eq!(app.bib_file_entries.len(), 1);
    let mutated_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(!mutated_content.contains("vaswani2017"));

    // Undo via Ctrl+Z
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));

    // Verify exact previous content restored on disk
    let restored_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert_eq!(restored_content, initial_bib);
    assert_eq!(app.bib_file_entries.len(), 2);
    assert!(app.status_message.contains("Undone: Delete bib entry"));

    // Second undo has nothing left
    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.status_message, "Nothing to undo");
}

#[test]
fn test_note_insert_and_undo_restores_exact_paper_draft() {
    let dir = tempdir().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let draft_path = root.join("paper_draft.tex");

    let initial_draft = "\\documentclass{article}\n\\begin{document}\n\\section{Introduction}\nOriginal draft content here.\n\\end{document}\n";
    std::fs::write(draft_path.as_std_path(), initial_draft).unwrap();

    let mut app = App::new(Some(root.clone()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("attention.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::Normal;

    // Save reader note onto draft
    app.save_reader_note("Important insight about multi-head attention");

    let mutated_draft = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert!(mutated_draft.contains("Important insight about multi-head attention"));

    // Undo via CommandId::Undo dispatch
    app.dispatch(CommandId::Undo);

    // Verify exact content restored on disk and in memory
    let restored_draft = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert_eq!(restored_draft, initial_draft);
    assert_eq!(app.paper_draft_content, initial_draft);
    assert!(app.status_message.contains("Undone: Capture note"));
}

#[test]
fn test_undo_via_command_palette() {
    let dir = tempdir().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let bib_path = root.join("references.bib");

    let initial_bib = "@article{test2024,\n  title = {Test Paper}\n}";
    std::fs::write(bib_path.as_std_path(), initial_bib).unwrap();

    let mut app = App::new(Some(root.clone()));
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::References;
    app.active_ref_pane = RefPane::LeftBib;
    app.load_project_references_bib();

    app.delete_selected_bib_entry();
    assert!(
        std::fs::read_to_string(bib_path.as_std_path())
            .unwrap()
            .is_empty()
    );

    // Open palette
    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::CommandPalette);

    // Filter for "undo"
    for c in "undo".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }

    // Execute selected command (Enter)
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);

    // Verify restored
    let restored = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert_eq!(restored, initial_bib);
    assert!(app.status_message.contains("Undone: Delete bib entry"));
}

#[test]
fn test_undo_empty_sets_nothing_to_undo() {
    let dir = tempdir().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();

    let mut app = App::new(Some(root));
    app.input_mode = InputMode::Normal;

    app.handle_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::CONTROL));
    assert_eq!(app.status_message, "Nothing to undo");
}

#[test]
fn test_undo_without_project_root() {
    let mut app = App::new(None);
    app.dispatch(CommandId::Undo);
    assert_eq!(app.status_message, "No active project loaded");
}
