use super::super::*;
use camino::Utf8Path;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sil_core::SourceDocument;
use tempfile::tempdir;

#[test]
fn test_picker_select_introduction_section() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");

    let initial_tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Introductory paragraphs on transformers.

\section{Methodology}
Architecture details and experimental setup.
\end{document}
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("vaswani2017.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;
    app.reading_md_content = Some("Source markdown content...".to_string());

    // 1. Press 'n' to open capture note modal
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ModalCaptureNote);
    assert!(app.capture_note_buffer.is_empty());

    // 2. Type note and press Enter
    let note_text = "Self-attention mechanism replaces recurrence entirely";
    for c in note_text.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    // 3. App transitions to NoteSectionPicker with populated draft sections
    assert_eq!(app.input_mode, InputMode::NoteSectionPicker);
    assert_eq!(
        app.note_picker_sections,
        vec![
            Some("Introduction".to_string()),
            Some("Methodology".to_string()),
            None,
        ]
    );
    assert_eq!(app.note_picker_selected, 0);

    // 4. Press Enter with Introduction selected (index 0)
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    // 5. Restores ReadingSourceMd and updates status
    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
    assert!(app.status_message.contains("Note captured into section Introduction"));

    // 6. Verify paper_draft.tex has idea block right under Introduction
    let updated_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert!(updated_tex.contains("% # -- X -- #"));
    assert!(updated_tex.contains("% from: vaswani2017.pdf"));
    assert!(updated_tex.contains(note_text));
    assert!(updated_tex.contains("tags=from-source"));

    let intro_pos = updated_tex.find(r"\section{Introduction}").unwrap();
    let note_pos = updated_tex.find(note_text).unwrap();
    let method_pos = updated_tex.find(r"\section{Methodology}").unwrap();
    assert!(
        intro_pos < note_pos && note_pos < method_pos,
        "Note block should be inserted within Introduction section before Methodology"
    );

    // 7. Verify parsed block has section_id == Some("Introduction")
    let blocks = sil_latex::parse_idea_blocks(&updated_tex);
    assert_eq!(blocks.len(), 1);
    assert_eq!(blocks[0].section_id.as_deref(), Some("Introduction"));
    assert_eq!(blocks[0].author_type, "human");
    assert_eq!(blocks[0].tags, vec!["from-source"]);
}

#[test]
fn test_picker_esc_cancels_without_mutation() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");

    let initial_tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Introductory paragraphs.
\end{document}
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("paper.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;

    // Open note modal and enter text
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
    for c in "Cancelled thought".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::NoteSectionPicker);

    // Press Esc in section picker
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
    assert_eq!(app.status_message, "Note capture cancelled.");
    assert!(app.pending_note_text.is_empty());
    assert!(app.note_picker_sections.is_empty());

    // paper_draft.tex remains untouched
    let final_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert_eq!(final_tex, initial_tex);
}

#[test]
fn test_picker_select_end_of_draft() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");

    let initial_tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Intro text.
\section{Results}
Results text.
\end{document}
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("attention.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;

    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
    let note = "Global observation for future discussion";
    for c in note.chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::NoteSectionPicker);
    assert_eq!(app.note_picker_sections.len(), 3); // Introduction, Results, End of draft

    // Navigate to "End of draft" (index 2)
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 1);
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 2);

    // Confirm selection
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
    assert!(app.status_message.contains("Parked note from attention.pdf"));

    let updated_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert!(updated_tex.contains(note));
    assert!(updated_tex.contains("% from: attention.pdf"));
    assert!(updated_tex.contains("tags=from-source"));
}

#[test]
fn test_picker_navigation_and_clamping() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");

    let initial_tex = r#"\section{Sec1}
\section{Sec2}
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("paper.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;

    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
    for c in "Note".chars() {
        app.handle_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::empty()));
    }
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::NoteSectionPicker);
    assert_eq!(app.note_picker_selected, 0);

    // Up at 0 does not underflow
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 0);
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 0);

    // Down moves down
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 1);
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 2);

    // Down at max bound (2) does not overflow
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 2);
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 2);

    // Up moves back
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
    assert_eq!(app.note_picker_selected, 1);
}

#[test]
fn test_empty_note_noop_does_not_open_picker() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");

    let initial_tex = "\\section{Sec1}\nText.\n";
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("paper.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;

    // Press 'n', submit empty buffer
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ModalCaptureNote);
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
    assert!(app.note_picker_sections.is_empty());
    let final_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert_eq!(final_tex, initial_tex);
}
