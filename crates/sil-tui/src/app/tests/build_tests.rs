use super::super::*;
use crate::app::jobs::parse_latex_error_location;

#[test]
fn parser_extracts_tectonic_and_pdflatex_locations() {
    assert_eq!(
        parse_latex_error_location("error: paper_draft.tex:17: Undefined control sequence"),
        Some(("paper_draft.tex".to_string(), 17))
    );
    assert_eq!(
        parse_latex_error_location("./sections/methods.tex:9:3: Missing }"),
        Some(("./sections/methods.tex".to_string(), 9))
    );
}

#[test]
fn build_line_jump_clamps_to_draft_length() {
    let mut app = App::new(None);
    app.paper_draft_content = "a\nb\nc".to_string();
    app.jump_to_draft_line("paper_draft.tex", 99);
    assert_eq!(app.active_tab, ActiveTab::PaperDraft);
    assert_eq!(app.paper_scroll_offset, 2);
}

#[test]
fn build_is_spawned_and_deduplicated() {
    let dir = tempfile::tempdir().unwrap();
    let root = camino::Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let mut app = App::new(Some(root));
    app.loaded_config = Some(sil_core::Config::default());
    app.run_build_job();
    assert!(app.in_flight_build);
    app.run_build_job();
    assert!(app.in_flight_build);
}
