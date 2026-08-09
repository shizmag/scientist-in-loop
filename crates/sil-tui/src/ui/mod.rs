//! Ratatui UI drawing code for `sil-tui`.

pub(crate) mod dashboard;
pub(crate) mod draft;
pub(crate) mod modals;
pub(crate) mod references;
pub(crate) mod settings;
pub(crate) mod sources;

#[cfg(test)]
mod tests;

use dashboard::draw_dashboard;
use draft::{draw_editing_paper_popup, draw_paper_draft};
use modals::{
    draw_confirm_delete_source, draw_editing_popup, draw_help_overlay, draw_job_history,
    draw_modal_add_author, draw_modal_add_grant, draw_modal_add_source_link, draw_modal_picker,
    draw_modal_rename_source,
};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Paragraph, Tabs},
};
use references::draw_references;
use settings::draw_settings;
use sources::{draw_sources, draw_viewing_source_refs};

use crate::app::{ActiveTab, App, InputMode};

/// Main UI draw loop.
pub fn draw(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Header & Tabs
            Constraint::Min(0),    // Content Area
            Constraint::Length(3), // Status & Footer
        ])
        .split(frame.area());

    draw_header(frame, app, chunks[0]);

    match app.active_tab {
        ActiveTab::Dashboard => draw_dashboard(frame, app, chunks[1]),
        ActiveTab::Sources => draw_sources(frame, app, chunks[1]),
        ActiveTab::References => draw_references(frame, app, chunks[1]),
        ActiveTab::PaperDraft => draw_paper_draft(frame, app, chunks[1]),
        ActiveTab::Settings => draw_settings(frame, app, chunks[1]),
    }

    draw_footer(frame, app, chunks[2]);

    // Modals overlay
    match app.input_mode {
        InputMode::HelpOverlay => draw_help_overlay(frame, app),
        InputMode::Editing => draw_editing_popup(frame, app),
        InputMode::EditingPaper => draw_editing_paper_popup(frame, app),
        InputMode::ModalPicker => draw_modal_picker(frame, app),
        InputMode::ModalAddAuthor => draw_modal_add_author(frame, app),
        InputMode::ModalAddGrant => draw_modal_add_grant(frame, app),
        InputMode::ModalAddSourceLink => draw_modal_add_source_link(frame, app),
        InputMode::ModalRenameSource => draw_modal_rename_source(frame, app),
        InputMode::ConfirmDeleteSource => draw_confirm_delete_source(frame, app),
        InputMode::JobHistory => draw_job_history(frame, app),
        InputMode::ViewingSourceRefs | InputMode::SearchingViewingRefs => {
            draw_viewing_source_refs(frame, app)
        }
        InputMode::SearchingRefs
        | InputMode::SearchingBib
        | InputMode::ReadingSourceMd
        | InputMode::Normal => {}
    }
}

fn draw_header(frame: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<Line> = ActiveTab::ALL
        .iter()
        .map(|t| {
            let style = if app.active_tab == *t {
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD | Modifier::UNDERLINED)
            } else {
                Style::default().fg(Color::Reset)
            };
            Line::from(vec![Span::styled(t.title(), style)])
        })
        .collect();

    let project_label = if let Some(ref root) = app.project_root {
        format!(" Project: {} ", root.file_name().unwrap_or("active"))
    } else {
        " Global Mode ".to_string()
    };

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Blue))
                .title(Span::styled(
                    " 🔬 scientist-in-loop ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
                .title_alignment(Alignment::Left)
                .title(Span::styled(
                    project_label,
                    Style::default().fg(Color::Yellow),
                )),
        )
        .select(app.active_tab as usize)
        .highlight_style(Style::default().fg(Color::Cyan));

    frame.render_widget(tabs, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let dirty_indicator = if app.dirty { " [UNSAVED CHANGES] " } else { "" };

    let msg_style = if app.status_message.contains("saved") || app.status_message.starts_with('✓')
    {
        Style::default()
            .fg(Color::Green)
            .add_modifier(Modifier::BOLD)
    } else if app.status_message.contains("cannot")
        || app.status_message.contains("Error")
        || app.status_message.contains("failed")
        || app.status_message.contains('⚠')
    {
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD)
    } else if app.status_message.starts_with('⏳')
        || app.status_message.contains("Hydrating")
        || app.status_message.contains("Parsing")
        || app.status_message.contains("fetching")
        || app.status_message.contains("Recomputing")
        || app.status_message.starts_with('ℹ')
    {
        Style::default()
            .fg(Color::Cyan)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    let mode = app.current_help_mode();
    let hints_str = match mode {
        crate::app::HelpMode::Dashboard => {
            "[?] Help | [J] Jobs | [1-5] Tabs | [Ctrl+S] Save | [q] Quit"
        }
        crate::app::HelpMode::SourcesList => {
            "[?] Help | [e/E] Parse | [a] Fetch | [J] Jobs | [Enter] Read | [v] Refs | [d] Del"
        }
        crate::app::HelpMode::ReadingSourceMd => {
            "[?] Help | [j/k] Scroll | [PgUp/PgDn] Page | [Esc] Exit"
        }
        crate::app::HelpMode::ViewingSourceRefs => {
            "[?] Help | [c] Add Bib | [a] Add All | [Space] Mark | [/] Filter"
        }
        crate::app::HelpMode::ReferencesLeft => {
            "[?] Help | [Tab] Switch Pane | [P] Promote | [J] Jobs | [/] Search | [Del] Delete"
        }
        crate::app::HelpMode::ReferencesRight => {
            "[?] Help | [Tab] Pane | [p] Add Bib | [m] Sort | [X] Recompute | [J] Jobs"
        }
        crate::app::HelpMode::PaperDraft => {
            "[?] Help | [e] Edit | [v] $EDITOR | [J] Jobs | [1-5] Tabs"
        }
        crate::app::HelpMode::Settings => {
            "[?] Help | [e] Edit | [a] Add | [d] Delete | [J] Jobs | [u] Use Cache"
        }
        crate::app::HelpMode::JobHistory => {
            "[?] Help | [j/k] Navigate | [Enter/r] Retry failed | [Esc] Close"
        }
        _ => "[?] / [F1] Help Overlay | [Esc] Cancel",
    };

    let footer_text = Paragraph::new(Line::from(vec![
        Span::styled(&app.status_message, msg_style),
        Span::styled(
            dirty_indicator,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("  |  Hints: {hints_str}"),
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" Status & Key Hints (Press '?' / F1 for Help) "),
    );

    frame.render_widget(footer_text, area);
}

pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
