//! Unit tests for PR-O1: First-Run Wizard in `sil-tui`.

use super::super::*;
use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_wizard_initialization_when_no_project_root() {
    let app = App::new(None);
    assert_eq!(app.input_mode, InputMode::Wizard);
    assert_eq!(app.saved_input_mode, InputMode::Wizard);
    assert!(app.project_root.is_none());
    assert_eq!(app.wizard_state.selected_menu_index, 0);
    assert!(app.status_message.contains("Welcome to scientist-in-loop"));
}

#[test]
fn test_wizard_navigation_and_shortcuts() {
    let mut app = App::new(None);
    assert_eq!(app.wizard_state.selected_menu_index, 0);

    // Down arrow or 'j' moves menu index down
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.wizard_state.selected_menu_index, 1);

    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::empty()));
    assert_eq!(app.wizard_state.selected_menu_index, 2);

    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.wizard_state.selected_menu_index, 3);

    // Clamps at 3
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.wizard_state.selected_menu_index, 3);

    // Up arrow or 'k' moves menu index up
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.wizard_state.selected_menu_index, 2);

    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::empty()));
    assert_eq!(app.wizard_state.selected_menu_index, 1);

    // Ctrl+J / Ctrl+K
    app.handle_key(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    assert_eq!(app.wizard_state.selected_menu_index, 2);

    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
    assert_eq!(app.wizard_state.selected_menu_index, 1);
}

#[test]
fn test_wizard_quick_select_numbers() {
    let mut app = App::new(None);

    // Pressing '2' opens OpenPath sub-mode
    app.handle_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::WizardOpenPath);

    // Esc cancels back to Wizard
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Wizard);

    // Pressing '3' opens CreateProject sub-mode
    app.handle_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::WizardCreateProject);

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Wizard);

    // Pressing '4' runs Doctor
    app.handle_key(KeyEvent::new(KeyCode::Char('4'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::WizardDoctorReport);
    assert!(!app.wizard_state.doctor_checks.is_empty());

    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Wizard);
}

#[test]
fn test_wizard_quit_on_q() {
    let mut app = App::new(None);
    assert!(!app.should_quit);

    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));
    assert!(app.should_quit);
}

#[test]
fn test_wizard_help_overlay() {
    let mut app = App::new(None);
    assert_eq!(app.input_mode, InputMode::Wizard);
    assert_eq!(app.current_help_mode(), HelpMode::Wizard);

    // Press '?'
    app.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::HelpOverlay);
    assert_eq!(app.current_help_mode(), HelpMode::Wizard);

    // Press Esc to dismiss
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Wizard);
}

#[test]
fn test_wizard_recent_projects_selection_and_opening() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let sil_dir = root.join(".sil");
    std::fs::create_dir_all(sil_dir.as_std_path()).unwrap();
    std::fs::write(sil_dir.join("config.yaml").as_std_path(), b"").unwrap();

    let mut app = App::new(None);
    app.global_settings.recent_projects = vec![root.clone()];
    app.wizard_state
        .refresh_recent_projects(&app.global_settings);

    assert_eq!(app.wizard_state.recent_projects.len(), 1);
    assert_eq!(app.wizard_state.selected_menu_index, 0);

    // Press Enter to open selected recent project
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.project_root, Some(root));
}

#[test]
fn test_wizard_recent_projects_filters_missing_paths() {
    let mut app = App::new(None);
    app.global_settings.recent_projects = vec![
        Utf8PathBuf::from("/nonexistent/path/alpha"),
        Utf8PathBuf::from("/nonexistent/path/beta"),
    ];
    app.wizard_state
        .refresh_recent_projects(&app.global_settings);

    // Non-existent paths are filtered out
    assert!(app.wizard_state.recent_projects.is_empty());

    // Pressing Enter when no recents exist gives a user-friendly error
    app.wizard_state.selected_menu_index = 0;
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Wizard);
    assert_eq!(
        app.last_user_error.as_ref().map(|e| e.code),
        Some("project.not_found")
    );
}

#[test]
fn test_wizard_open_path_success() {
    let dir = tempfile::tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let sil_dir = root.join(".sil");
    std::fs::create_dir_all(sil_dir.as_std_path()).unwrap();
    std::fs::write(sil_dir.join("config.yaml").as_std_path(), b"").unwrap();

    let mut app = App::new(None);
    app.input_mode = InputMode::WizardOpenPath;
    app.wizard_state.open_path_buffer = root.to_string();

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.project_root, Some(root.clone()));
    assert!(app.global_settings.recent_projects.contains(&root));
}

#[test]
fn test_wizard_open_path_nonexistent_returns_user_error() {
    let mut app = App::new(None);
    app.input_mode = InputMode::WizardOpenPath;
    app.wizard_state.open_path_buffer = "/nonexistent/invalid/dir".to_string();

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::WizardOpenPath);
    assert_eq!(
        app.last_user_error.as_ref().map(|e| e.code),
        Some("project.not_found")
    );
    assert!(app.project_root.is_none());
}

#[test]
fn test_wizard_create_project_flow() {
    let parent_dir = tempfile::tempdir().unwrap();
    let target = Utf8PathBuf::from_path_buf(parent_dir.path().to_path_buf())
        .unwrap()
        .join("my-fresh-paper");

    let mut app = App::new(None);
    app.input_mode = InputMode::WizardCreateProject;
    app.wizard_state.create_project_buffer = target.to_string();

    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert_eq!(app.project_root, Some(target.clone()));
    assert!(target.join(".sil/config.yaml").exists());
    assert!(target.join(".sil/db.sqlite").exists());
    assert!(target.join("paper_draft.tex").exists());
    assert!(target.join("references.bib").exists());
    assert!(app.status_message.contains("Created and opened project"));
}

#[test]
fn test_wizard_doctor_report_scrolling_and_exit() {
    let mut app = App::new(None);
    app.run_wizard_doctor();
    assert_eq!(app.input_mode, InputMode::WizardDoctorReport);
    assert_eq!(app.wizard_state.doctor_scroll_offset, 0);

    // Down key scrolls
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    if app.wizard_state.doctor_checks.len() > 1 {
        assert_eq!(app.wizard_state.doctor_scroll_offset, 1);
    }

    // Up key scrolls back
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.wizard_state.doctor_scroll_offset, 0);

    // Esc returns to Wizard
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Wizard);
}
