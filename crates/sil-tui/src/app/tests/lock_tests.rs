use crate::app::{App, CommandId, InputMode};
use camino::Utf8PathBuf;
use sil_core::{ProjectPaths, WorkspaceLock, read_lock, write_lock};
use tempfile::tempdir;

fn setup_temp_project() -> (tempfile::TempDir, Utf8PathBuf) {
    let dir = tempdir().unwrap();
    let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
    let paths = ProjectPaths::new(&root);
    std::fs::create_dir_all(paths.sil_dir().as_std_path()).unwrap();
    std::fs::create_dir_all(root.join("sources").as_std_path()).unwrap();

    let cfg = sil_core::Config::default();
    std::fs::write(paths.config().as_std_path(), cfg.to_yaml().unwrap()).unwrap();
    std::fs::write(
        paths.paper_draft().as_std_path(),
        "\\documentclass{article}\n\\begin{document}\nInitial\n\\end{document}\n",
    )
    .unwrap();
    std::fs::write(root.join("references.bib").as_std_path(), "").unwrap();

    (dir, root)
}

#[test]
fn test_startup_acquires_lock_in_project() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    let app = App::new(Some(root.clone()));
    assert!(app.active_lock_conflict.is_none());
    assert!(app.lock_holder_banner.is_none());

    let lock = read_lock(&paths).unwrap().expect("lock must exist");
    assert_eq!(lock.holder, "tui");
    assert_eq!(lock.op, "session");
    assert_eq!(lock.pid, Some(std::process::id()));

    app.cleanup_lock();
    assert!(read_lock(&paths).unwrap().is_none());
}

#[test]
fn test_startup_detects_live_lock_conflict() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    // Pre-write a lock with current PID (alive) and holder "mcp"
    let mcp_lock = WorkspaceLock {
        holder: "mcp".to_string(),
        pid: Some(std::process::id()),
        started: 100,
        op: "edit-section".to_string(),
    };
    write_lock(&paths, &mcp_lock).unwrap();

    let app = App::new(Some(root.clone()));
    assert!(app.active_lock_conflict.is_some());
    let banner = app.lock_holder_banner.as_ref().expect("banner must be set");
    assert!(banner.contains("mcp is edit-section"));
    assert_eq!(
        app.last_user_error.as_ref().map(|e| e.code),
        Some("lock.held")
    );

    // Cleanup should not clear someone else's lock
    app.cleanup_lock();
    let lock_after = read_lock(&paths).unwrap().expect("mcp lock should remain");
    assert_eq!(lock_after.holder, "mcp");
}

#[test]
fn test_startup_treats_dead_pid_as_stale() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    // Pre-write lock with dead PID
    let dead_lock = WorkspaceLock {
        holder: "mcp".to_string(),
        pid: Some(99_999_999),
        started: 100,
        op: "edit-section".to_string(),
    };
    write_lock(&paths, &dead_lock).unwrap();

    let app = App::new(Some(root.clone()));
    assert!(app.active_lock_conflict.is_none());
    assert!(app.lock_holder_banner.is_none());

    let lock = read_lock(&paths).unwrap().expect("lock must exist");
    assert_eq!(lock.holder, "tui");
    assert_eq!(lock.op, "session");
    assert_eq!(lock.pid, Some(std::process::id()));
}

#[test]
fn test_mutating_dispatch_without_confirm_does_not_write_when_held() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    // Inject lock held by live process (mcp)
    let mcp_lock = WorkspaceLock {
        holder: "mcp".to_string(),
        pid: Some(std::process::id()),
        started: 100,
        op: "estimate".to_string(),
    };
    write_lock(&paths, &mcp_lock).unwrap();

    let mut app = App::new(Some(root.clone()));
    assert!(app.active_lock_conflict.is_some());

    // Modify draft in memory
    app.paper_draft_content = "MODIFIED CONTENT".to_string();
    app.dirty = true;

    // Dispatch SaveAll without prior confirmation
    app.dispatch(CommandId::SaveAll);

    // Draft on disk should NOT be modified
    let disk_content = std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap();
    assert!(!disk_content.contains("MODIFIED CONTENT"));
    assert!(disk_content.contains("Initial"));

    // App state flags
    assert_eq!(
        app.last_user_error.as_ref().map(|e| e.code),
        Some("lock.held")
    );
    assert!(app.confirm_lock_override);
    assert!(app.status_message.contains("Warning:"));

    // Second dispatch (confirm) should now write!
    app.dispatch(CommandId::SaveAll);
    let disk_content_after = std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap();
    assert_eq!(disk_content_after, "MODIFIED CONTENT");
    assert!(!app.dirty);
}

#[test]
fn test_mutating_cite_source_blocked_then_confirmed() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    // Add a source file
    let pdf_path = root.join("sources").join("paper.pdf");
    std::fs::write(pdf_path.as_std_path(), b"%PDF-1.4 dummy content").unwrap();

    // Inject lock held by live process
    let mcp_lock = WorkspaceLock {
        holder: "mcp".to_string(),
        pid: Some(std::process::id()),
        started: 100,
        op: "hydrate".to_string(),
    };
    write_lock(&paths, &mcp_lock).unwrap();

    let mut app = App::new(Some(root.clone()));
    assert!(!app.sources.is_empty());

    // First attempt to cite source should block
    app.dispatch(CommandId::CiteSource);

    let bib_content = std::fs::read_to_string(root.join("references.bib").as_std_path()).unwrap();
    assert!(bib_content.is_empty());
    assert!(app.confirm_lock_override);
    assert_eq!(
        app.last_user_error.as_ref().map(|e| e.code),
        Some("lock.held")
    );

    // Second attempt should write to references.bib
    app.dispatch(CommandId::CiteSource);
    let bib_content_after =
        std::fs::read_to_string(root.join("references.bib").as_std_path()).unwrap();
    assert!(!bib_content_after.is_empty());
}

#[test]
fn test_save_reader_note_blocked_then_confirmed() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    let pdf_path = root.join("sources").join("sample.pdf");
    std::fs::write(pdf_path.as_std_path(), b"%PDF-1.4 sample").unwrap();

    let mcp_lock = WorkspaceLock {
        holder: "mcp".to_string(),
        pid: Some(std::process::id()),
        started: 100,
        op: "edit-section".to_string(),
    };
    write_lock(&paths, &mcp_lock).unwrap();

    let mut app = App::new(Some(root.clone()));
    assert!(!app.sources.is_empty());

    // Attempt to save reader note
    app.save_reader_note("Important discovery");

    let draft_content = std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap();
    assert!(!draft_content.contains("Important discovery"));
    assert!(app.confirm_lock_override);

    // Second attempt confirms and writes
    app.save_reader_note("Important discovery");
    let draft_after = std::fs::read_to_string(paths.paper_draft().as_std_path()).unwrap();
    assert!(draft_after.contains("Important discovery"));
}

#[test]
fn test_delete_source_blocked_then_confirmed() {
    let (_dir, root) = setup_temp_project();
    let paths = ProjectPaths::new(&root);

    let src_path = root.join("sources").join("todelete.pdf");
    std::fs::write(src_path.as_std_path(), b"%PDF-1.4 dummy").unwrap();

    let mcp_lock = WorkspaceLock {
        holder: "mcp".to_string(),
        pid: Some(std::process::id()),
        started: 100,
        op: "edit-section".to_string(),
    };
    write_lock(&paths, &mcp_lock).unwrap();

    let mut app = App::new(Some(root.clone()));
    assert_eq!(app.sources.len(), 1);
    app.input_mode = InputMode::ConfirmDeleteSource;

    // First keypress to delete: blocked
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('y'),
        crossterm::event::KeyModifiers::NONE,
    ));

    assert!(src_path.is_file());
    assert!(app.confirm_lock_override);

    // Second keypress: confirmed and deleted
    app.input_mode = InputMode::ConfirmDeleteSource;
    app.handle_key(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('y'),
        crossterm::event::KeyModifiers::NONE,
    ));

    assert!(!src_path.is_file());
}
