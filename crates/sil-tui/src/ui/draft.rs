//! Paper draft view rendering for `sil-tui`.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Wrap},
};

use super::centered_rect;
use crate::app::App;

pub(crate) fn draw_paper_draft(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left Column: LaTeX Section Outline / Parser Tree
    let mut items = Vec::new();
    if app.paper_sections.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " (no sections / empty paper_draft.tex)",
            Style::default().fg(Color::Reset),
        ))));
    } else {
        for (idx, sec) in app.paper_sections.iter().enumerate() {
            let is_selected = app.paper_section_index == idx;
            let prefix = if is_selected { "► " } else { "  " };
            let style = if is_selected {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(
                    format!("[{}] ", sec.kind),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(&sec.title, style),
                Span::styled(
                    format!(" (L{})", sec.line_start),
                    Style::default().fg(Color::Cyan),
                ),
            ])));
        }
    }

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 📄 Manuscript Sections (↑/↓ to navigate) ");

    let left_list = List::new(items).block(left_block);
    frame.render_widget(left_list, chunks[0]);

    // Right Column: Section Content Viewer
    let (sec_title, body_text) = if !app.paper_sections.is_empty()
        && app.paper_section_index < app.paper_sections.len()
    {
        let sec = &app.paper_sections[app.paper_section_index];
        (
            format!(
                " Section: {} (Press 'e': edit, 'v': $EDITOR, PgUp/PgDn: scroll) ",
                sec.title
            ),
            sec.body.clone(),
        )
    } else if !app.paper_draft_content.is_empty() {
        (
            " paper_draft.tex (Full View — Press 'v' for $EDITOR) ".to_string(),
            app.paper_draft_content.clone(),
        )
    } else {
        (
            " Section Content ".to_string(),
            "No paper_draft.tex found or empty file.\nPress 'v' to launch external editor (nvim/helix) or 'e' to create draft.".to_string(),
        )
    };

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(sec_title);

    let paragraph = Paragraph::new(body_text)
        .block(right_block)
        .wrap(Wrap { trim: false })
        .scroll((app.paper_scroll_offset as u16, 0));

    frame.render_widget(paragraph, chunks[1]);
}

pub(crate) fn draw_editing_paper_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, area);

    let sec_title =
        if !app.paper_sections.is_empty() && app.paper_section_index < app.paper_sections.len() {
            format!(
                " Editing Section: {} (Enter: Confirm, Esc: Cancel) ",
                app.paper_sections[app.paper_section_index].title
            )
        } else {
            " Editing paper_draft.tex (Enter: Confirm, Esc: Cancel) ".to_string()
        };

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            sec_title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(app.paper_edit_buffer.as_str())
        .block(popup_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}
