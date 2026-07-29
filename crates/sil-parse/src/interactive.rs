//! Keyboard-friendly multi-select for unparsed PDFs.

use camino::Utf8PathBuf;
use sil_core::SilUi;

use crate::error::ParseError;

/// Pure selection events (testable without a TTY).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectionEvent {
    /// Move cursor up.
    Up,
    /// Move cursor down.
    Down,
    /// Toggle current item.
    Toggle,
    /// Select all.
    All,
    /// Select none.
    None,
    /// Confirm current selection.
    Confirm,
    /// Cancel (empty result).
    Cancel,
    /// Ignored key.
    Ignore,
}

/// Outcome of applying one selection event.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SelectionOutcome {
    /// Keep the UI open.
    Continue,
    /// User confirmed; indices of selected paths.
    Confirmed(Vec<usize>),
    /// User cancelled; no paths.
    Cancelled,
}

/// Apply one selection event to cursor + selection state.
///
/// Used by the TTY loop and unit-tested without terminal I/O.
pub fn apply_selection_event(
    event: SelectionEvent,
    selected: &mut [bool],
    cursor: &mut usize,
) -> SelectionOutcome {
    let n = selected.len();
    if n == 0 {
        return match event {
            SelectionEvent::Confirm | SelectionEvent::Cancel => SelectionOutcome::Cancelled,
            _ => SelectionOutcome::Continue,
        };
    }
    if *cursor >= n {
        *cursor = n - 1;
    }
    match event {
        SelectionEvent::Up => {
            *cursor = cursor.saturating_sub(1);
            SelectionOutcome::Continue
        }
        SelectionEvent::Down => {
            if *cursor + 1 < n {
                *cursor += 1;
            }
            SelectionOutcome::Continue
        }
        SelectionEvent::Toggle => {
            selected[*cursor] = !selected[*cursor];
            SelectionOutcome::Continue
        }
        SelectionEvent::All => {
            selected.fill(true);
            SelectionOutcome::Continue
        }
        SelectionEvent::None => {
            selected.fill(false);
            SelectionOutcome::Continue
        }
        SelectionEvent::Confirm => {
            let chosen: Vec<usize> = selected
                .iter()
                .enumerate()
                .filter_map(|(i, s)| if *s { Some(i) } else { None })
                .collect();
            SelectionOutcome::Confirmed(chosen)
        }
        SelectionEvent::Cancel => SelectionOutcome::Cancelled,
        SelectionEvent::Ignore => SelectionOutcome::Continue,
    }
}

/// Interactive multi-select over paths. Returns selected indices.
///
/// Interactive TTY: ↑/↓ move, space toggles, `a`/`n` all/none, Enter confirms, `q` cancels.
/// Non-interactive: selects all.
pub fn select_pdfs_interactive(
    paths: &[Utf8PathBuf],
    ui: &dyn SilUi,
) -> Result<Vec<usize>, ParseError> {
    if paths.is_empty() {
        ui.warn("No unparsed PDFs found in sources/.");
        return Ok(Vec::new());
    }
    if !ui.interactive() {
        ui.info(&format!(
            "Non-interactive mode: selecting all {} unparsed PDF(s).",
            paths.len()
        ));
        return Ok((0..paths.len()).collect());
    }

    select_with_console(paths, ui)
}

fn select_with_console(
    paths: &[Utf8PathBuf],
    ui: &dyn SilUi,
) -> Result<Vec<usize>, ParseError> {
    use console::{Key, Term};

    let term = Term::stdout();
    let mut selected: Vec<bool> = vec![true; paths.len()];
    let mut cursor: usize = 0;
    let mut painted = false;
    let frame_lines = paths.len() + 3; // blank + title + help + rows

    // Always restore cursor, even on panic/error mid-loop.
    struct CursorGuard<'a>(&'a Term);
    impl Drop for CursorGuard<'_> {
        fn drop(&mut self) {
            let _ = self.0.show_cursor();
        }
    }

    let _ = term.hide_cursor();
    let _guard = CursorGuard(&term);

    (|| -> Result<Vec<usize>, ParseError> {
        loop {
            if painted {
                let _ = term.clear_last_lines(frame_lines);
            }
            paint_frame(&term, paths, &selected, cursor)?;
            painted = true;

            let event = match term
                .read_key()
                .map_err(|e| ParseError::Message(e.to_string()))?
            {
                Key::ArrowUp | Key::Char('k') => SelectionEvent::Up,
                Key::ArrowDown | Key::Char('j') => SelectionEvent::Down,
                Key::Char(' ') => SelectionEvent::Toggle,
                Key::Char('a') | Key::Char('A') => SelectionEvent::All,
                Key::Char('n') | Key::Char('N') => SelectionEvent::None,
                Key::Enter => SelectionEvent::Confirm,
                Key::Escape | Key::Char('q') | Key::Char('Q') => SelectionEvent::Cancel,
                _ => SelectionEvent::Ignore,
            };

            match apply_selection_event(event, &mut selected, &mut cursor) {
                SelectionOutcome::Continue => {}
                SelectionOutcome::Confirmed(chosen) => {
                    if painted {
                        let _ = term.clear_last_lines(frame_lines);
                    }
                    ui.info(&format!(
                        "Selected {} of {} PDF(s).",
                        chosen.len(),
                        paths.len()
                    ));
                    return Ok(chosen);
                }
                SelectionOutcome::Cancelled => {
                    if painted {
                        let _ = term.clear_last_lines(frame_lines);
                    }
                    ui.warn("Selection cancelled.");
                    return Ok(Vec::new());
                }
            }
        }
    })()
}

fn paint_frame(
    term: &console::Term,
    paths: &[Utf8PathBuf],
    selected: &[bool],
    cursor: usize,
) -> Result<(), ParseError> {
    use console::style;

    let write = |s: &str| {
        term.write_line(s)
            .map_err(|e| ParseError::Message(e.to_string()))
    };

    write("")?;
    write(&format!("{}", style("Select PDFs to parse").cyan().bold()))?;
    write(&format!(
        "{}",
        style("↑/↓ move  ·  space toggle  ·  a all  ·  n none  ·  enter confirm  ·  q cancel")
            .dim()
    ))?;

    for (i, p) in paths.iter().enumerate() {
        let mark = if selected[i] {
            style("[x]").green().to_string()
        } else {
            style("[ ]").dim().to_string()
        };
        let name = p.file_name().unwrap_or(p.as_str());
        if i == cursor {
            write(&format!(
                "  {} {}  {}",
                style(">").cyan().bold(),
                mark,
                style(name).bold()
            ))?;
        } else {
            write(&format!("    {mark}  {name}"))?;
        }
    }
    Ok(())
}

