//! Modal popups and help overlay view rendering for `sil-tui`.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Row, Table,
    },
};

use super::centered_rect;
use crate::app::App;

pub(crate) fn draw_help_overlay(frame: &mut Frame, app: &App) {
    let mode = app.current_help_mode();
    let keymap = crate::app::keymap_for(mode);

    let area = centered_rect(75, 75, frame.area());
    frame.render_widget(Clear, area);

    let title_line = format!(
        " ❓ Keyboard Help: {} (Press Esc / '?' / F1 / Any key to close) ",
        mode.title()
    );

    let popup_block = Block::default()
        .title(Span::styled(
            title_line,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let rows: Vec<Row> = keymap
        .into_iter()
        .map(|(key, action)| {
            Row::new(vec![
                Span::styled(
                    key,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(action, Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Percentage(30), Constraint::Percentage(70)],
    )
    .header(
        Row::new(vec!["Key / Shortcut", "Action / Description"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
    )
    .block(popup_block);

    frame.render_widget(table, area);
}

pub(crate) fn draw_editing_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let popup_block = Block::default()
        .title(" Edit Value (Enter to confirm, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let input_p = Paragraph::new(app.input_buffer.as_str())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(popup_block);

    frame.render_widget(input_p, area);
}

pub(crate) fn draw_modal_picker(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = app
        .cache
        .co_authors
        .iter()
        .enumerate()
        .map(|(idx, ca)| {
            let style = if idx == app.cache_coauthor_index {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            ListItem::new(format!("{} <{}> - {}", ca.name, ca.email, ca.affiliation)).style(style)
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .title(" Select Co-Author from Cache (Enter: Select, 'n': Add New, Esc: Cancel) ")
                .borders(Borders::ALL)
                .border_type(BorderType::Double)
                .border_style(Style::default().fg(Color::Cyan)),
        )
        .highlight_style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        );

    frame.render_widget(list, area);
}

pub(crate) fn draw_modal_add_author(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 45, frame.area());
    frame.render_widget(Clear, area);

    let orcid_str = app.new_author.orcid.clone().unwrap_or_default();
    let fields = [
        ("Name", app.new_author.name.as_str()),
        ("Email", app.new_author.email.as_str()),
        ("Affiliation", app.new_author.affiliation.as_str()),
        ("ORCID iD", orcid_str.as_str()),
    ];

    let mut lines = Vec::new();
    for (idx, (label, val)) in fields.iter().enumerate() {
        let is_sel = idx == app.modal_field_index;
        let prefix = if is_sel { "► " } else { "  " };
        let style = if is_sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{prefix}{label:<12}: "), style),
            Span::styled(
                if val.is_empty() { "_" } else { val },
                Style::default().fg(Color::Cyan),
            ),
        ]));
    }

    let block = Block::default()
        .title(" Add New Co-Author (Tab to switch field, Enter to save, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Magenta));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

pub(crate) fn draw_modal_add_grant(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 40, frame.area());
    frame.render_widget(Clear, area);

    let fields = [
        ("Funder", app.new_grant.funder.as_str()),
        ("Grant Number", app.new_grant.grant_number.as_str()),
        ("Acknowledgment", app.new_grant.acknowledgment.as_str()),
    ];

    let mut lines = Vec::new();
    for (idx, (label, val)) in fields.iter().enumerate() {
        let is_sel = idx == app.modal_field_index;
        let prefix = if is_sel { "► " } else { "  " };
        let style = if is_sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{prefix}{label:<15}: "), style),
            Span::styled(
                if val.is_empty() { "_" } else { val },
                Style::default().fg(Color::Green),
            ),
        ]));
    }

    let block = Block::default()
        .title(" Add New Grant Requisites (Tab to switch field, Enter to save, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Green));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

pub(crate) fn draw_modal_add_source_link(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 25, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Fetch / download source (URL / DOI / arXiv) — Enter to start, Esc to cancel ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let paragraph = Paragraph::new(app.new_source_link_buffer.as_str())
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(block);

    frame.render_widget(paragraph, area);
}

pub(crate) fn draw_job_history(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 70, frame.area());
    frame.render_widget(Clear, area);

    let title = " Background Job History (↑/↓ navigate, Enter/r retry failed, Esc close) ";
    let block = Block::default()
        .title(Span::styled(
            title,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    if app.recent_job_outcomes.is_empty() {
        let paragraph = Paragraph::new("No background jobs recorded yet.")
            .style(Style::default().fg(Color::DarkGray))
            .block(block)
            .alignment(Alignment::Center);
        frame.render_widget(paragraph, area);
        return;
    }

    // Newest last in deque — show newest at bottom, reverse for list with newest first.
    let rows: Vec<ListItem> = app
        .recent_job_outcomes
        .iter()
        .enumerate()
        .rev()
        .map(|(idx, o)| {
            let is_sel = idx == app.selected_job_history_index;
            let status = if o.ok { "OK" } else { "FAIL" };
            let status_style = if o.ok {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
                    .fg(Color::LightRed)
                    .add_modifier(Modifier::BOLD)
            };
            let retry_hint = if !o.ok && o.retry_payload.is_some() {
                " [retryable]"
            } else {
                ""
            };
            let dur = o
                .duration_ms
                .map(|ms| format!(" {ms}ms"))
                .unwrap_or_default();
            let detail: String = o.detail.chars().take(90).collect();
            let prefix = if is_sel { "► " } else { "  " };
            let base_style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(Line::from(vec![
                Span::styled(prefix, base_style),
                Span::styled(
                    format!("[{}] ", o.kind.label()),
                    Style::default().fg(Color::Magenta),
                ),
                Span::styled(format!("{status}{retry_hint} "), status_style),
                Span::styled(format!("{} — ", o.label), base_style),
                Span::styled(detail, Style::default().fg(Color::DarkGray)),
                Span::styled(dur, Style::default().fg(Color::Cyan)),
            ]))
        })
        .collect();

    // Map selected index (0=oldest) to reverse list selection.
    let n = app.recent_job_outcomes.len();
    let list_sel = n
        .saturating_sub(1)
        .saturating_sub(app.selected_job_history_index);

    let list = List::new(rows).block(block).highlight_style(
        Style::default()
            .bg(Color::DarkGray)
            .add_modifier(Modifier::BOLD),
    );
    let mut state = ListState::default();
    state.select(Some(list_sel));
    frame.render_stateful_widget(list, area, &mut state);
}

pub(crate) fn draw_modal_rename_source(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 25, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Rename Source Title (Enter to save, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(app.rename_source_buffer.as_str())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(block);

    frame.render_widget(paragraph, area);
}

pub(crate) fn draw_modal_capture_note(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 25, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Capture note for draft (Enter to choose section, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let paragraph = Paragraph::new(app.capture_note_buffer.as_str())
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(block);

    frame.render_widget(paragraph, area);
}

pub(crate) fn draw_modal_note_section_picker(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 45, frame.area());
    frame.render_widget(Clear, area);

    let items: Vec<ListItem> = app
        .note_picker_sections
        .iter()
        .enumerate()
        .map(|(idx, sec)| {
            let is_sel = idx == app.note_picker_selected;
            let (prefix, label) = match sec {
                Some(title) => ("§ ", title.as_str()),
                None => ("• ", "[End of draft]"),
            };
            let style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::White)
            };
            let cursor = if is_sel { "► " } else { "  " };
            ListItem::new(format!("{cursor}{prefix}{label}")).style(style)
        })
        .collect();

    let block = Block::default()
        .title(" Select Target Section for Note (j/k: Navigate, Enter: Confirm, Esc: Cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let list = List::new(items).block(block);

    frame.render_widget(list, area);
}

pub(crate) fn draw_confirm_delete_source(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 25, frame.area());
    frame.render_widget(Clear, area);

    let filename = if !app.sources.is_empty() && app.selected_source_index < app.sources.len() {
        &app.sources[app.selected_source_index].filename
    } else {
        "selected source"
    };

    let text = vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "Are you sure you want to delete source '",
                Style::default().fg(Color::White),
            ),
            Span::styled(
                filename,
                Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
            ),
            Span::styled("'?", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(Span::styled(
            "Press 'y' or Enter to confirm, 'n' or Esc to cancel.",
            Style::default().fg(Color::Yellow),
        )),
    ];

    let block = Block::default()
        .title(" ⚠️ Confirm Delete Source ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Red));

    let paragraph = Paragraph::new(text)
        .block(block)
        .alignment(Alignment::Center);
    frame.render_widget(paragraph, area);
}

pub(crate) fn draw_command_palette(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 65, frame.area());
    frame.render_widget(Clear, area);

    let main_block = Block::default()
        .title(Span::styled(
            " ⌨ Command Palette (: / Ctrl+K) ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan));

    let inner_area = main_block.inner(area);
    frame.render_widget(main_block, area);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // Search input box
            Constraint::Min(0),    // Filtered command list
        ])
        .split(inner_area);

    // Search input
    let search_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Yellow))
        .title(" Filter (type to search title, aliases, description) ");

    let query_display = format!("> {}█", app.palette_filter);
    let search_p = Paragraph::new(query_display)
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )
        .block(search_block);
    frame.render_widget(search_p, chunks[0]);

    // Command list
    let filtered = app.filtered_commands();
    if filtered.is_empty() {
        let empty_msg = Paragraph::new("No matching commands found.")
            .style(Style::default().fg(Color::DarkGray))
            .alignment(Alignment::Center)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .border_style(Style::default().fg(Color::DarkGray)),
            );
        frame.render_widget(empty_msg, chunks[1]);
        return;
    }

    let items: Vec<ListItem> = filtered
        .iter()
        .enumerate()
        .map(|(idx, spec)| {
            let is_sel = idx == app.palette_selected_index;
            let availability = spec.is_available(app);

            let prefix = if is_sel { "► " } else { "  " };

            match availability {
                Ok(()) => {
                    let title_style = if is_sel {
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::White)
                    };
                    let key_style = Style::default().fg(Color::Cyan);
                    let desc_style = Style::default().fg(Color::DarkGray);

                    let key_text = if spec.default_keys.is_empty() {
                        "".to_string()
                    } else {
                        format!(" [{}]", spec.default_keys)
                    };

                    ListItem::new(Line::from(vec![
                        Span::styled(prefix, title_style),
                        Span::styled(spec.title, title_style),
                        Span::styled(key_text, key_style),
                        Span::styled(format!(" — {}", spec.description), desc_style),
                    ]))
                }
                Err(reason) => {
                    let title_style = if is_sel {
                        Style::default()
                            .fg(Color::DarkGray)
                            .add_modifier(Modifier::BOLD)
                    } else {
                        Style::default().fg(Color::DarkGray)
                    };
                    let reason_style = Style::default().fg(Color::LightRed);
                    let desc_style = Style::default().fg(Color::DarkGray);

                    ListItem::new(Line::from(vec![
                        Span::styled(prefix, title_style),
                        Span::styled(spec.title, title_style),
                        Span::styled(format!(" (disabled: {reason})"), reason_style),
                        Span::styled(format!(" — {}", spec.description), desc_style),
                    ]))
                }
            }
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Blue))
                .title(" Commands (Enter: Run, ↑/↓/Tab: Navigate, Esc: Close) "),
        )
        .highlight_style(
            Style::default()
                .bg(Color::DarkGray)
                .add_modifier(Modifier::BOLD),
        );

    let mut state = ListState::default();
    state.select(Some(app.palette_selected_index));
    frame.render_stateful_widget(list, chunks[1], &mut state);
}

