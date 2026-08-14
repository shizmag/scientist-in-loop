use crate::app::{App, CommandId};
use camino::Utf8PathBuf;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use sil_core::{Config, ProjectPaths};
use std::thread::sleep;
use std::time::Duration;
use tempfile::tempdir;

fn setup_temp_project() -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let paths = ProjectPaths::new(&root);
    std::fs::create_dir_all(paths.sil_dir().as_std_path()).unwrap();
    std::fs::create_dir_all(root.join("sources").as_std_path()).unwrap();

    let cfg = Config::default();
    std::fs::write(paths.config().as_std_path(), cfg.to_yaml().unwrap()).unwrap();
    std::fs::write(
        paths.paper_draft().as_std_path(),
        "\\documentclass{article}\n\\begin{document}\nInitial Tex\n\\end{document}\n",
    )
    .unwrap();
    std::fs::write(
        root.join("references.bib").as_std_path(),
        "@article{orig, title={Original Title}}\n",
    )
    .unwrap();

    (dir, root)
}

#[test]
fn test_dirty_and_newer_mtime_save_blocked_without_confirm() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    let mut app = App::new(Some(root.clone()));
    assert_eq!(
        app.paper_draft_content,
        "\\documentclass{article}\n\\begin{document}\nInitial Tex\n\\end{document}\n"
    );
    assert!(!app.dirty);
    assert!(app.disk_conflict_banner.is_none());
    assert!(!app.confirm_disk_overwrite);

    // Modify in-memory TUI state and mark dirty
    app.paper_draft_content =
        "\\documentclass{article}\n\\begin{document}\nTUI Edited\n\\end{document}\n".to_string();
    app.dirty = true;

    // External change on disk with newer mtime
    sleep(Duration::from_millis(50));
    std::fs::write(
        paths.paper_draft().as_std_path(),
        "\\documentclass{article}\n\\begin{document}\nDisk Modified Externally\n\\end{document}\n",
    )
    .unwrap();

    // First save attempt should be BLOCKED
    app.save_all();

    // Invariants check:
    // 1. TUI remains dirty
    assert!(app.dirty);
    // 2. confirm_disk_overwrite is now true
    assert!(app.confirm_disk_overwrite);
    // 3. Conflict banner is displayed
    assert!(app.disk_conflict_banner.is_some());
    let banner = app.disk_conflict_banner.as_ref().unwrap();
    assert!(banner.contains("Disk changed externally"));
    // 4. last_user_error has code conflict.disk_newer
    assert_eq!(
        app.last_user_error.as_ref().map(|e| e.code),
        Some("conflict.disk_newer")
    );
    // 5. Disk content was NOT overwritten
    let disk_content = std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap();
    assert_eq!(
        disk_content,
        "\\documentclass{article}\n\\begin{document}\nDisk Modified Externally\n\\end{document}\n"
    );
}

#[test]
fn test_confirming_save_overwrites_and_updates_snapshot() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    let mut app = App::new(Some(root.clone()));
    app.paper_draft_content =
        "\\documentclass{article}\n\\begin{document}\nTUI Overwrite\n\\end{document}\n".to_string();
    app.dirty = true;

    // External modification
    sleep(Duration::from_millis(50));
    std::fs::write(
        paths.paper_draft().as_std_path(),
        "\\documentclass{article}\n\\begin{document}\nExternal\n\\end{document}\n",
    )
    .unwrap();

    // 1st save -> blocked
    app.save_all();
    assert!(app.confirm_disk_overwrite);
    assert!(app.dirty);

    // 2nd save -> user confirmed overwrite
    app.save_all();
    assert!(!app.dirty);
    assert!(!app.confirm_disk_overwrite);
    assert!(app.disk_conflict_banner.is_none());
    assert!(!app.disk_conflict_pending);

    // Disk content is now TUI Overwrite
    let disk_content = std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap();
    assert_eq!(
        disk_content,
        "\\documentclass{article}\n\\begin{document}\nTUI Overwrite\n\\end{document}\n"
    );

    // Immediate next save does NOT trigger false conflict
    app.dirty = true;
    app.save_all();
    assert!(!app.dirty);
    assert!(app.disk_conflict_banner.is_none());
}

#[test]
fn test_non_dirty_and_newer_mtime_reload_cleanly() {
    let (_dir, root) = setup_temp_project();

    let mut app = App::new(Some(root.clone()));
    assert_eq!(app.bib_file_entries.len(), 1);
    assert!(!app.dirty);

    // External modification to references.bib
    sleep(Duration::from_millis(50));
    std::fs::write(
        root.join("references.bib").as_std_path(),
        "@article{orig, title={Original}}\n@article{new1, title={New Ext Entry}}\n",
    )
    .unwrap();

    // check_disk_conflicts when not dirty does not block
    let conflict = app.check_disk_conflicts();
    assert!(!conflict);
    assert!(app.disk_conflict_banner.is_none());
    assert!(app.status_message.contains("Disk changed externally"));

    // Reloading via command
    app.dispatch(CommandId::Reload);
    assert_eq!(app.bib_file_entries.len(), 2);
    assert!(app.disk_conflict_banner.is_none());
    assert!(!app.confirm_disk_overwrite);
    assert!(!app.dirty);

    // Subsequent check finds no conflict
    assert!(!app.check_disk_conflicts());
}

#[test]
fn test_config_external_modification_detected() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    let mut app = App::new(Some(root.clone()));
    app.local_settings.title = "TUI Title".to_string();
    app.dirty = true;

    // External modification to .sil/config.yaml
    sleep(Duration::from_millis(50));
    let mut ext_cfg = Config::default();
    ext_cfg.project.title = "Disk Title".to_string();
    std::fs::write(paths.config().as_std_path(), ext_cfg.to_yaml().unwrap()).unwrap();

    // Check conflict
    assert!(app.check_disk_conflicts());
    assert!(app.disk_conflict_banner.is_some());

    // Save blocked
    app.save_all();
    assert!(app.confirm_disk_overwrite);
    assert!(app.dirty);

    // Reload drops dirty and loads disk config
    app.dispatch(CommandId::Reload);
    assert_eq!(app.local_settings.title, "");
    assert_eq!(
        app.loaded_config.as_ref().unwrap().project.title,
        "Disk Title"
    );
    assert!(!app.dirty);
    assert!(app.disk_conflict_banner.is_none());
}

#[test]
fn test_dismiss_conflict_banner_keep_tui() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    let mut app = App::new(Some(root.clone()));
    app.paper_draft_content = "TUI Draft".to_string();
    app.dirty = true;

    // External change
    sleep(Duration::from_millis(50));
    std::fs::write(paths.paper_draft().as_std_path(), "Disk Draft").unwrap();

    // Conflict triggered
    assert!(app.check_disk_conflicts());
    assert!(app.disk_conflict_banner.is_some());

    // Dismiss banner via Esc key in normal mode
    app.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::empty()));
    assert!(app.disk_conflict_banner.is_none());
    assert!(!app.should_quit);
    assert!(app.status_message.contains("Dismissed conflict banner"));

    // Next save is still protected and requires confirm
    app.save_all();
    assert!(app.confirm_disk_overwrite);
    assert_eq!(
        std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap(),
        "Disk Draft"
    );

    // Second save overwrites
    app.save_all();
    assert_eq!(
        std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap(),
        "TUI Draft"
    );
    assert!(!app.dirty);
}
