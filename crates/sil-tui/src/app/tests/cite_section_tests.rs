use super::super::*;
use camino::Utf8Path;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sil_core::SourceDocument;
use tempfile::tempdir;

#[test]
fn test_reading_source_md_keymap_includes_c() {
    let keymap = keymap_for(HelpMode::ReadingSourceMd);
    assert!(
        keymap
            .iter()
            .any(|(k, v)| *k == "c" && v.contains("Insert \\cite into draft section")),
        "Keymap for ReadingSourceMd must document 'c'"
    );
}

#[test]
fn test_picker_select_introduction_section_and_undo() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");
    let bib_path = root.join("references.bib");

    let initial_tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Introductory paragraphs on transformers.

\section{Methodology}
Architecture details and experimental setup.
\end{document}
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();
    std::fs::write(bib_path.as_std_path(), "").unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("vaswani2017.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;
    app.reading_md_content = Some("Source markdown content...".to_string());
    app.reload_paper_draft();

    // 1. Press 'c' to initiate CiteIntoSection
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));

    // 2. references.bib was upserted with draft entry
    let bib_content = std::fs::read_to_string(bib_path.as_std_path()).unwrap();
    assert!(bib_content.contains("% [sil: tui-added]"));
    assert!(bib_content.contains("@"));

    // 3. App transitions to CiteSectionPicker
    assert_eq!(app.input_mode, InputMode::CiteSectionPicker);
    assert_eq!(
        app.cite_picker_sections,
        vec!["Introduction".to_string(), "Methodology".to_string()]
    );
    assert_eq!(app.cite_picker_selected, 0);
    assert!(!app.pending_cite_key.is_empty());
    let cite_key = app.pending_cite_key.clone();

    // 4. Press Enter with Introduction selected (index 0)
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    // 5. Restores ReadingSourceMd and updates status
    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
    assert_eq!(
        app.status_message,
        format!("Cited {cite_key} in section Introduction")
    );

    // 6. Verify paper_draft.tex has \cite in Introduction section
    let updated_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert!(
        updated_tex.contains(&format!("~\\cite{{{cite_key}}}"))
            || updated_tex.contains(&format!("\\cite{{{cite_key}}}"))
    );

    let intro_pos = updated_tex.find(r"\section{Introduction}").unwrap();
    let cite_pos = updated_tex.find(&format!("\\cite{{{cite_key}}}")).unwrap();
    let method_pos = updated_tex.find(r"\section{Methodology}").unwrap();
    assert!(
        intro_pos < cite_pos && cite_pos < method_pos,
        "Citation should be inserted within Introduction section before Methodology"
    );

    // 7. Test Undo
    app.dispatch(CommandId::Undo);
    assert_eq!(app.status_message, "Undone: Cite source in draft section");
    let undone_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert_eq!(undone_tex, initial_tex);
}

#[test]
fn test_cite_picker_esc_cancels_without_mutation() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");
    let bib_path = root.join("references.bib");

    let initial_tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
Introductory paragraphs.
\end{document}
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();
    std::fs::write(bib_path.as_std_path(), "").unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("paper.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;
    app.reload_paper_draft();

    // Press 'c' to open picker
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::CiteSectionPicker);

    // Press Esc in section picker
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
    assert_eq!(app.status_message, "Cite into section cancelled.");
    assert!(app.pending_cite_key.is_empty());
    assert!(app.cite_picker_sections.is_empty());

    // paper_draft.tex remains untouched
    let final_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert_eq!(final_tex, initial_tex);
}

#[test]
fn test_cite_picker_navigation_and_clamping() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");
    let bib_path = root.join("references.bib");

    let initial_tex = r#"\section{Sec1}
Text 1.
\section{Sec2}
Text 2.
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();
    std::fs::write(bib_path.as_std_path(), "").unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("paper.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;
    app.reload_paper_draft();

    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::CiteSectionPicker);
    assert_eq!(app.cite_picker_selected, 0);

    // Up at 0 does not underflow
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.cite_picker_selected, 0);
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
    assert_eq!(app.cite_picker_selected, 0);

    // Down moves down
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.cite_picker_selected, 1);
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.cite_picker_selected, 1); // Clamped at len - 1 (1)

    // Up moves back
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
    assert_eq!(app.cite_picker_selected, 0);

    // Down to Sec2 and confirm
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.cite_picker_selected, 1);
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    let updated_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert!(updated_tex.contains("Text 2.~\\cite{"));
}

#[test]
fn test_second_cite_in_same_section_is_noop() {
    let dir = tempdir().unwrap();
    let root = Utf8Path::from_path(dir.path()).unwrap();
    let draft_path = root.join("paper_draft.tex");
    let bib_path = root.join("references.bib");

    let initial_tex = r#"\section{Sec1}
Text 1.
"#;
    std::fs::write(draft_path.as_std_path(), initial_tex).unwrap();
    std::fs::write(bib_path.as_std_path(), "").unwrap();

    let mut app = App::new(Some(root.to_path_buf()));
    let doc = SourceDocument::new(camino::Utf8PathBuf::from("paper.pdf"));
    app.sources.push(doc);
    app.selected_source_index = 0;
    app.input_mode = InputMode::ReadingSourceMd;
    app.reload_paper_draft();

    // 1st cite into Sec1
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
    let cite_key = app.pending_cite_key.clone();
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    let once_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert!(once_tex.contains(&format!("\\cite{{{cite_key}}}")));

    // 2nd cite into Sec1
    app.handle_key(KeyEvent::new(KeyCode::Char('c'), KeyModifiers::empty()));
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    assert_eq!(
        app.status_message,
        format!("Already cited {cite_key} in section Sec1")
    );
    let twice_tex = std::fs::read_to_string(draft_path.as_std_path()).unwrap();
    assert_eq!(once_tex, twice_tex);
}
