use super::super::*;
use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn app_with_root() -> (tempfile::TempDir, App) {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    (dir, App::new(Some(root)))
}

#[test]
fn open_last_review_missing_dir_shows_empty_state() {
    let (_dir, mut app) = app_with_root();
    app.open_last_review();
    assert_eq!(app.input_mode, InputMode::EstimateReport);
    assert_eq!(
        app.estimate_report_content.as_deref(),
        Some("no reviews yet — run Estimate")
    );
}

#[test]
fn open_last_review_loads_markdown_fixture() {
    let (dir, mut app) = app_with_root();
    let review_dir = dir.path().join(".sil/reviews/review_fixture");
    std::fs::create_dir_all(&review_dir).unwrap();
    std::fs::write(
        review_dir.join("report.md"),
        "# Fixture Review\n\nFinding text\n",
    )
    .unwrap();

    app.open_last_review();
    assert_eq!(app.input_mode, InputMode::EstimateReport);
    assert!(
        app.estimate_report_content
            .as_deref()
            .unwrap()
            .contains("Finding text")
    );
}

#[test]
fn run_estimate_does_not_open_source_reader_or_dirty_draft() {
    let (_dir, mut app) = app_with_root();
    app.run_estimate_job();
    assert!(app.reading_md_content.is_none());
    assert!(!app.dirty);
    assert!(app.in_flight_estimate);
}

#[test]
fn estimate_report_scrolls_and_closes() {
    let (_dir, mut app) = app_with_root();
    app.estimate_report_content = Some("report".to_string());
    app.input_mode = InputMode::EstimateReport;
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.estimate_report_scroll_offset, 1);
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.estimate_report_content.is_none());
}
