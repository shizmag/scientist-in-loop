//! First-run wizard UI rendering for `sil-tui`.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Row, Table},
};

use super::centered_rect;
use crate::app::{App, InputMode};

/// Draw first-run wizard view and its sub-mode modals.
pub fn draw_wizard(frame: &mut Frame, app: &App, area: Rect) {
    if app.input_mode == InputMode::WizardDoctorReport {
        draw_doctor_report(frame, app, area);
        return;
    }

    let popup_area = centered_rect(75, 75, area);
    frame.render_widget(Clear, popup_area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " 🔬 scientist-in-loop: First-Run Wizard ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ))
        .title_alignment(Alignment::Center);

    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .margin(2)
        .constraints([
            Constraint::Length(3), // Welcome banner
            Constraint::Length(1), // Separator
            Constraint::Min(8),    // Menu options
            Constraint::Length(3), // Selected option info card
            Constraint::Length(1), // Key hints footer
        ])
        .split(popup_area);

    frame.render_widget(main_block, popup_area);

    // Welcome banner
    let banner_text = Paragraph::new(vec![
        Line::from(vec![
            Span::styled("Welcome to ", Style::default().fg(Color::White)),
            Span::styled(
                "scientist-in-loop",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                " — agent-friendly scientific paper workspace.",
                Style::default().fg(Color::White),
            ),
        ]),
        Line::from(vec![Span::styled(
            "Select an option below to open an existing project, create a new one, or check dependencies:",
            Style::default().fg(Color::DarkGray),
        )]),
    ])
    .alignment(Alignment::Left);
    frame.render_widget(banner_text, chunks[0]);

    // Menu options list
    let selected_idx = app.wizard_state.selected_menu_index;

    let recent_label = if app.wizard_state.recent_projects.is_empty() {
        "1. Open Recent Project  (no recent projects found)".to_string()
    } else {
        let count = app.wizard_state.recent_projects.len();
        let r_idx = app.wizard_state.selected_recent_index;
        let path = &app.wizard_state.recent_projects[r_idx];
        if count == 1 {
            format!("1. Open Recent Project: {path}")
        } else {
            format!(
                "1. Open Recent Project: [{}/{}] {} (◄/► to cycle)",
                r_idx + 1,
                count,
                path
            )
        }
    };

    let menu_items = [
        (0, recent_label),
        (1, "2. Open Directory / Path".to_string()),
        (2, "3. Create New Project".to_string()),
        (3, "4. Run System Doctor".to_string()),
    ];

    let items: Vec<ListItem> = menu_items
        .into_iter()
        .map(|(idx, label)| {
            let is_selected = idx == selected_idx;
            let (prefix, style) = if is_selected {
                (
                    " ▶ ",
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                ("   ", Style::default().fg(Color::White))
            };

            ListItem::new(Line::from(vec![
                Span::styled(
                    prefix,
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(label, style),
            ]))
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Blue))
            .title(Span::styled(
                " Menu Options (Press 1-4 or Up/Down + Enter) ",
                Style::default().fg(Color::Cyan),
            )),
    );
    frame.render_widget(list, chunks[2]);

    // Description info card
    let info_text = match selected_idx {
        0 => {
            if app.wizard_state.recent_projects.is_empty() {
                "No previous projects found in global settings history."
            } else {
                "Open a previously visited scientific paper workspace."
            }
        }
        1 => "Specify a directory containing a .sil/ project (config.yaml, sources, draft).",
        2 => "Scaffold a new paper workspace with LaTeX templates, SQLite DB, and skills.",
        3 => "Inspect host environment dependencies (git, python3, uv, latex engines, pdf parser).",
        _ => "",
    };

    let info_block = Paragraph::new(Line::from(vec![
        Span::styled(
            "ℹ  ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(info_text, Style::default().fg(Color::White)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray)),
    );
    frame.render_widget(info_block, chunks[3]);

    // Key hints footer
    let hints_text = Paragraph::new(Line::from(vec![
        Span::styled(
            "[1-4] Quick Select  |  [↑/↓ or j/k] Navigate  |  [Enter] Select  |  [q] Quit",
            Style::default().fg(Color::DarkGray),
        ),
    ]))
    .alignment(Alignment::Center);
    frame.render_widget(hints_text, chunks[4]);

    // Sub-modal overlays
    match app.input_mode {
        InputMode::WizardOpenPath => draw_wizard_open_path(frame, app),
        InputMode::WizardCreateProject => draw_wizard_create_project(frame, app),
        _ => {}
    }
}

fn draw_wizard_open_path(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 25, frame.area());
    frame.render_widget(Clear, area);

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow))
        .title(Span::styled(
            " 📂 Open Directory / Project Path (Enter to Open, Esc to Back) ",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2), // Hint
            Constraint::Length(3), // Input field
        ])
        .split(area);

    frame.render_widget(popup_block, area);

    let hint = Paragraph::new(
        "Enter directory path containing .sil/ workspace (relative or absolute):",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, layout[0]);

    let input_text = format!("{}_", app.wizard_state.open_path_buffer);
    let input_p = Paragraph::new(input_text)
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Cyan)),
        );
    frame.render_widget(input_p, layout[1]);
}

fn draw_wizard_create_project(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 25, frame.area());
    frame.render_widget(Clear, area);

    let popup_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            " 🪄 Create New Project (Enter to Create, Esc to Back) ",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

    let layout = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints([
            Constraint::Length(2), // Hint
            Constraint::Length(3), // Input field
        ])
        .split(area);

    frame.render_widget(popup_block, area);

    let hint = Paragraph::new(
        "Enter folder name or path (e.g. 'my-paper'):",
    )
    .style(Style::default().fg(Color::DarkGray));
    frame.render_widget(hint, layout[0]);

    let input_text = format!("{}_", app.wizard_state.create_project_buffer);
    let input_p = Paragraph::new(input_text)
        .style(
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Green)),
        );
    frame.render_widget(input_p, layout[1]);
}

fn draw_doctor_report(frame: &mut Frame, app: &App, area: Rect) {
    let popup_area = centered_rect(85, 85, area);
    frame.render_widget(Clear, popup_area);

    let main_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            " 🩺 System Doctor: Host Environment Report (Esc / Enter / q to Back) ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ));

    let checks = &app.wizard_state.doctor_checks;
    let offset = app.wizard_state.doctor_scroll_offset;

    let rows: Vec<Row> = checks
        .iter()
        .skip(offset)
        .map(|c| {
            let (status_symbol, status_style) = if c.ok {
                ("✔", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))
            } else if is_soft_check(&c.name) {
                ("·", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD))
            } else {
                ("✖", Style::default().fg(Color::Red).add_modifier(Modifier::BOLD))
            };

            let hint_str = c.hint.as_deref().unwrap_or("");
            let detail_text = if hint_str.is_empty() {
                c.detail.clone()
            } else {
                format!("{}\n  ↳ Hint: {}", c.detail, hint_str)
            };

            Row::new(vec![
                Span::styled(status_symbol, status_style),
                Span::styled(&c.name, Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
                Span::styled(detail_text, Style::default().fg(Color::White)),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Length(3),
            Constraint::Length(16),
            Constraint::Min(40),
        ],
    )
    .header(
        Row::new(vec!["", "Component", "Status & Diagnostic Details"]).style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
        ),
    )
    .block(main_block);

    frame.render_widget(table, popup_area);
}

fn is_soft_check(name: &str) -> bool {
    name == "tectonic"
        || name == "pdflatex"
        || name == "latexmk"
        || name == "uv"
        || name == "marker"
        || name == "dense_rag"
}
