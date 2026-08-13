//! Unit tests for empty states, stalled states, next-command chips, and CommandId dispatches.

use super::super::*;
use crate::ui::draw;
use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use sil_core::SourceDocument;

fn render_app_to_string(app: &mut App) -> String {
    let backend = TestBackend::new(240, 60);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, app)).unwrap();
    let buffer = terminal.backend().buffer();
    let mut out = String::new();
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            let cell = &buffer[(x, y)];
            out.push_str(cell.symbol());
        }
        out.push('\n');
    }
    out
}

fn normalize_whitespace(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

#[test]
fn test_empty_sources_shows_fetch_copy() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::Sources;
    assert!(app.sources.is_empty());

    let rendered = render_app_to_string(&mut app);
    let norm = normalize_whitespace(&rendered);
    assert!(
        norm.contains("No sources found. Drop a PDF/MD in sources/ or Fetch by DOI/URL [a: Add Source]"),
        "Rendered output should contain empty sources fetch copy. Output:\n{rendered}"
    );
}

#[test]
fn test_unparsed_sources_shows_parse_copy() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::Sources;

    let mut doc1 = SourceDocument::new(Utf8PathBuf::from("sources/doc1.pdf"));
    doc1.parsed = false;
    let mut doc2 = SourceDocument::new(Utf8PathBuf::from("sources/doc2.pdf"));
    doc2.parsed = false;

    app.sources.push(doc1);
    app.sources.push(doc2);

    let rendered = render_app_to_string(&mut app);
    let norm = normalize_whitespace(&rendered);
    assert!(
        norm.contains("2 unparsed — [e: Parse selected / Shift+E: Parse all]"),
        "Rendered output should contain unparsed count banner. Output:\n{rendered}"
    );
}

#[test]
fn test_non_empty_sources_does_not_show_empty_state() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::Sources;

    let mut doc = SourceDocument::new(Utf8PathBuf::from("sources/parsed_doc.pdf"));
    doc.parsed = true;
    doc.title = Some("Parsed Document Title".to_string());
    app.sources.push(doc);

    let rendered = render_app_to_string(&mut app);
    let norm = normalize_whitespace(&rendered);
    assert!(
        !norm.contains("No sources found"),
        "Rendered output must not show 'No sources found' when sources are present. Output:\n{rendered}"
    );
    assert!(
        !norm.contains("Drop a PDF/MD"),
        "Rendered output must not show 'Drop a PDF/MD' when sources are present. Output:\n{rendered}"
    );
    assert!(norm.contains("Parsed Document Title"));
}

#[test]
fn test_draft_with_no_sections_shows_external_editor_copy() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::PaperDraft;
    app.paper_sections.clear();
    app.paper_draft_content.clear();

    let rendered = render_app_to_string(&mut app);
    let norm = normalize_whitespace(&rendered);
    assert!(
        norm.contains(r"Draft has no \section yet — [o: Open in $EDITOR]"),
        "Rendered output should contain external editor copy for draft with no sections. Output:\n{rendered}"
    );
}

#[test]
fn test_empty_references_right_pane_shows_extract_refs_copy() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::References;
    app.source_references.clear();

    let rendered = render_app_to_string(&mut app);
    let norm = normalize_whitespace(&rendered);
    assert!(
        norm.contains("No references extracted. Select a parsed source in Sources tab and press 'v' to view/extract refs."),
        "Rendered output should contain references extraction guidance. Output:\n{rendered}"
    );
}

#[test]
fn test_empty_dashboard_digest_shows_refresh_copy() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::Dashboard;
    app.dashboard.digest_publications.clear();

    let rendered = render_app_to_string(&mut app);
    let norm = normalize_whitespace(&rendered);
    assert!(
        norm.contains("No digest entries. Configure topic query in Settings (Tab 5) or press ':' for palette to Refresh digest."),
        "Rendered dashboard should contain digest empty copy. Output:\n{rendered}"
    );
}

#[test]
fn test_sources_empty_enter_and_a_dispatches_add_source() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
    let mut app = App::new(Some(root));
    app.active_tab = ActiveTab::Sources;
    assert!(app.sources.is_empty());

    // Pressing 'a' enters ModalAddSourceLink
    app.handle_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ModalAddSourceLink);

    // Cancel modal
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);

    // Pressing 'Enter' on empty sources also enters ModalAddSourceLink
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ModalAddSourceLink);
}

#[test]
fn test_sources_unparsed_e_and_shift_e_dispatches_parse() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(temp_dir.path().to_path_buf()).unwrap();
    let mut app = App::new(Some(root));
    app.active_tab = ActiveTab::Sources;

    let mut doc1 = SourceDocument::new(Utf8PathBuf::from("sources/doc1.pdf"));
    let id1: sil_core::SourceId = "doc1".to_string().into();
    doc1.id = id1.clone();
    doc1.parsed = false;
    let mut doc2 = SourceDocument::new(Utf8PathBuf::from("sources/doc2.pdf"));
    let id2: sil_core::SourceId = "doc2".to_string().into();
    doc2.id = id2.clone();
    doc2.parsed = false;

    app.sources.push(doc1);
    app.sources.push(doc2);
    app.selected_source_index = 0;

    // Press 'e' -> dispatches ParseSelected
    app.handle_key(KeyEvent::new(KeyCode::Char('e'), KeyModifiers::empty()));
    assert!(app.in_flight_parse_ids.contains(&id1));

    // Press 'E' / Shift+E -> dispatches ParseAll
    app.handle_key(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
    assert!(app.in_flight_parse_ids.contains(&id2));
}

#[test]
fn test_draft_o_and_v_dispatches_open_external_editor() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    app.active_tab = ActiveTab::PaperDraft;
    assert!(!app.pending_external_editor);

    // Press 'o'
    app.handle_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::empty()));
    assert!(app.pending_external_editor);

    app.pending_external_editor = false;

    // Press 'v'
    app.handle_key(KeyEvent::new(KeyCode::Char('v'), KeyModifiers::empty()));
    assert!(app.pending_external_editor);
}

#[test]
fn test_dispatch_refresh_digest_and_open_external_editor() {
    let mut app = App::new(None);

    // Dispatch OpenExternalEditor
    app.pending_external_editor = false;
    app.dispatch(CommandId::OpenExternalEditor);
    assert!(app.pending_external_editor);
    assert!(app.status_message.contains("Launching external editor"));

    // Dispatch RefreshDigest without query
    app.global_settings.digest_query.clear();
    app.local_settings.digest_query.clear();
    app.dispatch(CommandId::RefreshDigest);
    assert!(app.status_message.contains("No digest query configured"));
}
