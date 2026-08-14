use super::super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

#[test]
fn test_filter_parse_commands() {
    let mut app = App::new(None);
    app.palette_filter = "parse".to_string();

    let filtered = app.filtered_commands();
    assert!(!filtered.is_empty(), "Should match parse commands");

    let ids: Vec<CommandId> = filtered.iter().map(|c| c.id).collect();
    assert!(ids.contains(&CommandId::ParseSelected));
    assert!(ids.contains(&CommandId::ParseAll));

    assert!(!ids.contains(&CommandId::SaveAll));
    assert!(!ids.contains(&CommandId::Quit));
    assert!(!ids.contains(&CommandId::OpenPalette));
    assert!(!ids.contains(&CommandId::OpenHelp));
    assert!(!ids.contains(&CommandId::Reload));
}

#[test]
fn test_palette_searches_registry_titles_and_aliases() {
    let mut app = App::new(None);

    app.palette_filter = "cite source into draft section".to_string();
    assert!(
        app.filtered_commands()
            .iter()
            .any(|spec| spec.id == CommandId::CiteIntoSection)
    );

    app.palette_filter = "draft-note".to_string();
    assert!(
        app.filtered_commands()
            .iter()
            .any(|spec| spec.id == CommandId::CaptureNote)
    );
}

#[test]
fn test_active_tab_registry_stays_at_five_tabs() {
    assert_eq!(ActiveTab::ALL.len(), 5);
}

#[test]
fn test_esc_restores_previous_mode() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;
    assert_eq!(app.input_mode, InputMode::Normal);

    // Open palette from Normal mode
    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::CommandPalette);

    // Esc closes palette and restores Normal mode
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);

    // Set to ReadingSourceMd mode
    app.input_mode = InputMode::ReadingSourceMd;
    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::CommandPalette);

    // Esc closes palette and restores ReadingSourceMd mode
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::ReadingSourceMd);
}

#[test]
fn test_dispatch_save_all_matches_ctrl_s() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;

    // 1. Test Ctrl+S
    app.dirty = true;
    app.handle_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL));
    assert!(!app.dirty, "Ctrl+S should clear dirty flag");
    assert!(
        app.status_message.contains("saved") || app.status_message.contains('✓'),
        "Ctrl+S should update status message"
    );

    // 2. Test dispatch(CommandId::SaveAll)
    app.dirty = true;
    app.dispatch(CommandId::SaveAll);
    assert!(!app.dirty, "dispatch(SaveAll) should clear dirty flag");
    assert!(
        app.status_message.contains("saved") || app.status_message.contains('✓'),
        "dispatch(SaveAll) should update status message"
    );
}

#[test]
fn test_palette_does_not_quit_on_q() {
    let mut app = App::new(None);
    app.dispatch(CommandId::OpenPalette);
    assert_eq!(app.input_mode, InputMode::CommandPalette);

    // Press 'q'
    app.handle_key(KeyEvent::new(KeyCode::Char('q'), KeyModifiers::empty()));

    assert!(!app.should_quit, "'q' in palette should not quit app");
    assert_eq!(app.palette_filter, "q");
    assert_eq!(app.input_mode, InputMode::CommandPalette);
}

#[test]
fn test_colon_and_ctrl_k_open_palette() {
    let mut app = App::new(None);
    app.input_mode = InputMode::Normal;

    // Test ':' from normal mode
    app.handle_key(KeyEvent::new(KeyCode::Char(':'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::CommandPalette);

    // Close palette
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);

    // Test Ctrl+K from normal mode
    app.handle_key(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::CONTROL));
    assert_eq!(app.input_mode, InputMode::CommandPalette);
}

#[test]
fn test_palette_navigation_and_clamping() {
    let mut app = App::new(None);
    app.dispatch(CommandId::OpenPalette);

    let total = app.filtered_commands().len();
    assert!(total > 2);
    assert_eq!(app.palette_selected_index, 0);

    // Down arrow moves selection down
    app.handle_key(KeyEvent::new(KeyCode::Down, KeyModifiers::empty()));
    assert_eq!(app.palette_selected_index, 1);

    // Ctrl+N moves selection down
    app.handle_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::CONTROL));
    assert_eq!(app.palette_selected_index, 2);

    // Tab moves selection down
    app.handle_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::empty()));
    assert_eq!(app.palette_selected_index, 3);

    // Up arrow moves selection up
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.palette_selected_index, 2);

    // Ctrl+P moves selection up
    app.handle_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL));
    assert_eq!(app.palette_selected_index, 1);

    // BackTab moves selection up
    app.handle_key(KeyEvent::new(KeyCode::BackTab, KeyModifiers::empty()));
    assert_eq!(app.palette_selected_index, 0);

    // Clamping on Up at 0 stays at 0
    app.handle_key(KeyEvent::new(KeyCode::Up, KeyModifiers::empty()));
    assert_eq!(app.palette_selected_index, 0);
}

#[test]
fn test_palette_enter_runs_command() {
    let mut app = App::new(None);
    app.dispatch(CommandId::OpenPalette);

    // Filter to "quit"
    app.palette_filter = "quit".to_string();
    let filtered = app.filtered_commands();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, CommandId::Quit);

    app.palette_selected_index = 0;
    app.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::empty()));

    assert!(
        app.should_quit,
        "Enter on Quit command should set should_quit"
    );
}

#[test]
fn test_command_id_as_str_and_display() {
    assert_eq!(CommandId::SaveAll.as_str(), "save_all");
    assert_eq!(CommandId::OpenPalette.as_str(), "open_palette");
    assert_eq!(CommandId::Quit.as_str(), "quit");
    assert_eq!(CommandId::OpenHelp.as_str(), "open_help");
    assert_eq!(CommandId::Reload.as_str(), "reload");
    assert_eq!(CommandId::OpenJobHistory.as_str(), "open_job_history");
    assert_eq!(CommandId::ParseSelected.as_str(), "parse_selected");
    assert_eq!(CommandId::ParseAll.as_str(), "parse_all");
    assert_eq!(CommandId::AddSourceLink.as_str(), "add_source_link");
    assert_eq!(CommandId::OpenSource.as_str(), "open_source");
    assert_eq!(CommandId::CiteSource.as_str(), "cite_source");
    assert_eq!(CommandId::CaptureNote.as_str(), "capture_note");
    assert_eq!(CommandId::RefreshDigest.as_str(), "refresh_digest");
    assert_eq!(
        CommandId::OpenExternalEditor.as_str(),
        "open_external_editor"
    );
    assert_eq!(CommandId::RepairDb.as_str(), "repair_db");

    assert_eq!(format!("{}", CommandId::SaveAll), "save_all");
    assert_eq!(format!("{}", CommandId::RefreshDigest), "refresh_digest");
    assert_eq!(
        format!("{}", CommandId::OpenExternalEditor),
        "open_external_editor"
    );
    assert_eq!(format!("{}", CommandId::RepairDb), "repair_db");
}

#[test]
fn test_command_spec_availability() {
    let app_without_root = App::new(None);

    let all = all_commands();
    let parse_cmd = all
        .iter()
        .find(|c| c.id == CommandId::ParseSelected)
        .unwrap();
    let quit_cmd = all.iter().find(|c| c.id == CommandId::Quit).unwrap();
    let reload_cmd = all.iter().find(|c| c.id == CommandId::Reload).unwrap();
    let refresh_digest_cmd = all
        .iter()
        .find(|c| c.id == CommandId::RefreshDigest)
        .unwrap();
    let editor_cmd = all
        .iter()
        .find(|c| c.id == CommandId::OpenExternalEditor)
        .unwrap();
    let repair_cmd = all.iter().find(|c| c.id == CommandId::RepairDb).unwrap();

    assert!(quit_cmd.is_available(&app_without_root).is_ok());
    assert!(parse_cmd.is_available(&app_without_root).is_err());
    assert!(reload_cmd.is_available(&app_without_root).is_err());
    assert!(refresh_digest_cmd.is_available(&app_without_root).is_err());
    assert!(editor_cmd.is_available(&app_without_root).is_err());
    assert!(repair_cmd.is_available(&app_without_root).is_err());
}

#[test]
fn test_repair_db_dispatch_and_confirm_modal() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path();
    let sil_dir = root.join(".sil");
    let sources_dir = root.join("sources");
    std::fs::create_dir_all(&sil_dir).unwrap();
    std::fs::create_dir_all(&sources_dir).unwrap();

    let doc_content = "# Doc\nContent";
    std::fs::write(sources_dir.join("paper.md"), doc_content).unwrap();

    let root_utf8 = camino::Utf8PathBuf::from_path_buf(root.to_path_buf()).unwrap();
    let mut app = App::new(Some(root_utf8));

    // Dispatch RepairDb enters confirm mode
    app.dispatch(CommandId::RepairDb);
    assert_eq!(app.input_mode, InputMode::ConfirmRepairDb);

    // Esc cancels
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.status_message.contains("cancelled"));

    // Dispatch RepairDb again and confirm with 'y'
    app.dispatch(CommandId::RepairDb);
    assert_eq!(app.input_mode, InputMode::ConfirmRepairDb);
    app.handle_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::empty()));
    assert_eq!(app.input_mode, InputMode::Normal);
    assert!(app.status_message.contains("repaired"));
}
