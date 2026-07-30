//! `sil-tui` library exposing Ratatui app state, UI rendering, and terminal event loop.

#![allow(clippy::collapsible_if, clippy::collapsible_match)]

pub mod app;
pub mod ui;


use std::io;
use std::time::Duration;
use anyhow::Result;
use camino::Utf8PathBuf;
use crossterm::{
    event::{self, Event, KeyCode, KeyModifiers},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};

pub use app::App;

/// Run the TUI application loop in terminal.
pub fn run_tui(project_root: Option<Utf8PathBuf>) -> Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut app = App::new(project_root);

    let res = run_app(&mut terminal, &mut app);

    // Restore terminal
    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    res
}

fn run_app<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| ui::draw(f, app))?;

        if app.pending_external_editor {
            app.pending_external_editor = false;
            open_external_editor(terminal, app)?;
            terminal.clear()?;
            continue;
        }

        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
                    break;
                }
                app.handle_key(key);
                if app.should_quit {
                    break;
                }
            }
        }
    }
    Ok(())
}

fn open_external_editor<B: ratatui::backend::Backend>(
    _terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    let Some(root) = app.project_root.clone() else {
        app.status_message = "External editor error: not inside a sil project root.".to_string();
        return Ok(());
    };

    let draft_path = root.join("paper_draft.tex");
    if !draft_path.is_file() {
        let initial_tex = r#"\documentclass{article}
\begin{document}
\section{Introduction}
% # -- X -- #
% TODO: write introduction
% # -- X -- #
\end{document}
"#;
        let _ = std::fs::write(draft_path.as_std_path(), initial_tex);
    }

    let editor = std::env::var("EDITOR")
        .or_else(|_| std::env::var("VISUAL"))
        .unwrap_or_else(|_| detect_available_editor());

    disable_raw_mode()?;
    execute!(io::stdout(), LeaveAlternateScreen)?;

    let status = std::process::Command::new(&editor)
        .arg(draft_path.as_std_path())
        .status();

    enable_raw_mode()?;
    execute!(io::stdout(), EnterAlternateScreen)?;

    match status {
        Ok(code) if code.success() => {
            app.reload_paper_draft();
            let paths = sil_core::ProjectPaths::new(&root);
            let _ = sil_latex::write_draft_sections_from_file(
                &draft_path,
                &paths.draft_sections_dir(),
            );
            if let Ok(db) = sil_db::SilDb::open(&paths.db()) {
                let ideas = sil_latex::parse_idea_blocks(&app.paper_draft_content);
                let _ = db.replace_todo_ideas(&ideas);
            }
            app.status_message =
                format!("✓ Returned from {editor}. paper_draft.tex reloaded & re-indexed.");
        }
        Ok(code) => {
            app.status_message = format!("Editor {editor} exited with code {code}.");
        }
        Err(e) => {
            app.status_message = format!("Failed to launch editor '{editor}': {e}");
        }
    }

    Ok(())
}

fn detect_available_editor() -> String {
    for cmd in ["nvim", "helix", "vim", "nano", "vi"] {
        if let Ok(output) = std::process::Command::new("which").arg(cmd).output() {
            if output.status.success() {
                return cmd.to_string();
            }
        }
    }
    "vim".to_string()
}
