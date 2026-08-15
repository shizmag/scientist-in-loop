use super::*;
use crate::app::{ActiveTab, App, InputMode};
use camino::Utf8PathBuf;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use sil_core::{AuthorDetails, GrantDetails, SourceDocument};

fn render_to_terminal(app: &mut App) {
    let backend = TestBackend::new(120, 40);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(f, app)).unwrap();
}

#[test]
fn test_centered_rect() {
    use ratatui::layout::Rect;
    let area = Rect::new(0, 0, 100, 100);
    let rect = centered_rect(50, 50, area);
    assert_eq!(rect.width, 50);
    assert_eq!(rect.height, 50);
}

#[test]
fn test_draw_dashboard_rendering() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Dashboard;
    render_to_terminal(&mut app);
}

#[test]
fn test_draws_shared_pr_v_manuscript_fixture() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::PaperDraft;
    app.reading_md_content =
        Some(include_str!("../../../../tests/fixtures/pr-v/paper_draft.tex").into());
    render_to_terminal(&mut app);
}

#[test]
fn test_draw_sources_rendering_empty_and_populated() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Sources;

    // Empty sources
    render_to_terminal(&mut app);

    // Populated sources
    let mut doc = SourceDocument::new(Utf8PathBuf::from("paper.pdf"));
    doc.title = Some("Test Title".to_string());
    doc.authors = Some("Author A".to_string());
    doc.venue = Some("NeurIPS".to_string());
    doc.year = Some(2024);
    doc.doi = Some("10.1234/5678".to_string());
    doc.abstract_text = Some("Abstract text preview".to_string());
    doc.references_text = Some("Ref 1\nRef 2\n".to_string());
    doc.parsed = true;

    app.sources.push(doc);
    render_to_terminal(&mut app);

    // Reading MD mode
    app.input_mode = InputMode::ReadingSourceMd;
    app.reading_md_content = Some("# Header\nMarkdown body".to_string());
    render_to_terminal(&mut app);
}

#[test]
fn test_draw_references_rendering() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::References;

    // Empty
    render_to_terminal(&mut app);

    // With bib and source references
    app.bib_file_entries = vec!["@article{a,\n title={A}\n}".to_string()];
    app.source_references = vec![sil_core::ReferenceEntry {
        id: "r1".to_string(),
        source_id: "s1".into(),
        ref_index: 1,
        raw_text: "Ref text".to_string(),
        title: Some("Title".to_string()),
        authors: Some("Author".to_string()),
        year: Some(2024),
        venue: Some("JMLR".to_string()),
        doi: Some("10.1234/5678".to_string()),
        arxiv_id: Some("2405.12345".to_string()),
        url: Some("https://example.com".to_string()),
    }];
    app.marked_ref_ids.insert("r1".to_string());
    render_to_terminal(&mut app);

    // Searching refs mode
    app.input_mode = InputMode::SearchingRefs;
    app.ref_search_query = "Ref".to_string();
    render_to_terminal(&mut app);

    // Searching bib mode
    app.input_mode = InputMode::SearchingBib;
    app.bib_search_query = "article".to_string();
    render_to_terminal(&mut app);
}

#[test]
fn test_draw_paper_draft_rendering() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::PaperDraft;

    // Empty
    render_to_terminal(&mut app);

    // With content and sections
    app.paper_draft_content = "\\section{Intro}\nBody text".to_string();
    app.paper_sections = sil_latex::split_tex_sections(&app.paper_draft_content);
    render_to_terminal(&mut app);
}

#[test]
fn test_draw_settings_rendering_empty_and_populated() {
    let mut app = App::new(None);
    app.active_tab = ActiveTab::Settings;

    // Empty state
    render_to_terminal(&mut app);

    // Populated cache and local settings
    app.cache.remember_co_author(AuthorDetails {
        name: "Cached Author".to_string(),
        email: "c@a.com".to_string(),
        affiliation: "MIT".to_string(),
        orcid: None,
    });
    app.cache.remember_grant(GrantDetails {
        funder: "NSF".to_string(),
        grant_number: "123".to_string(),
        acknowledgment: "Ack".to_string(),
    });
    app.local_settings.title = "Local Title".to_string();
    app.local_settings.notes = "Local Notes".to_string();
    app.local_settings.co_authors.push(AuthorDetails {
        name: "Local Author".to_string(),
        email: "l@a.com".to_string(),
        affiliation: "Stanford".to_string(),
        orcid: None,
    });
    app.local_settings.grants.push(GrantDetails {
        funder: "NIH".to_string(),
        grant_number: "456".to_string(),
        acknowledgment: "Ack 2".to_string(),
    });

    render_to_terminal(&mut app);
}

#[test]
fn test_draw_modal_popups_rendering() {
    let mut app = App::new(None);

    let modes = [
        InputMode::Editing,
        InputMode::EditingPaper,
        InputMode::ModalPicker,
        InputMode::ModalAddAuthor,
        InputMode::ModalAddGrant,
        InputMode::ModalAddSourceLink,
        InputMode::ModalRenameSource,
        InputMode::ModalCaptureNote,
        InputMode::NoteSectionPicker,
        InputMode::CiteSectionPicker,
        InputMode::ConfirmDeleteSource,
        InputMode::JobHistory,
        InputMode::ViewingSourceRefs,
        InputMode::HelpOverlay,
        InputMode::CommandPalette,
        InputMode::ProposalDiff,
    ];

    for mode in modes {
        app.input_mode = mode;
        render_to_terminal(&mut app);
    }
}

#[test]
fn test_draw_footer_status_styles() {
    let mut app = App::new(None);

    // Green saved message
    app.status_message = "✓ All saved successfully".to_string();
    app.dirty = true;
    render_to_terminal(&mut app);

    // Red error message
    app.status_message = "Error: cannot save file".to_string();
    app.dirty = false;
    render_to_terminal(&mut app);
}
