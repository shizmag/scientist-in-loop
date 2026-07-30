//! Ratatui UI drawing code for `sil-tui`.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph,
        Row, Table, Tabs,
    },
    Frame,
};

use crate::app::{ActiveTab, App, GlobalField, InputMode, LocalField};

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
        ActiveTab::GlobalSettings => draw_global_settings(frame, app, chunks[1]),
        ActiveTab::LocalSettings => draw_local_settings(frame, app, chunks[1]),
        ActiveTab::CoAuthorCache => draw_coauthor_cache(frame, app, chunks[1]),
        ActiveTab::GrantCache => draw_grant_cache(frame, app, chunks[1]),
    }

    draw_footer(frame, app, chunks[2]);

    // Modals overlay
    match app.input_mode {
        InputMode::Editing => draw_editing_popup(frame, app),
        InputMode::ModalPicker => draw_modal_picker(frame, app),
        InputMode::ModalAddAuthor => draw_modal_add_author(frame, app),
        InputMode::ModalAddGrant => draw_modal_add_grant(frame, app),
        InputMode::Normal => {}
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
                Style::default().fg(Color::DarkGray)
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
                    " 🔬 scientist-in-loop Settings ",
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
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

fn draw_global_settings(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left Column: Primary Author Details
    let orcid_str = app.global_settings.author.orcid.clone().unwrap_or_default();
    let author_fields = [
        ("Author Name", app.global_settings.author.name.as_str(), GlobalField::AuthorName),
        ("Email", app.global_settings.author.email.as_str(), GlobalField::AuthorEmail),
        ("Affiliation", app.global_settings.author.affiliation.as_str(), GlobalField::AuthorAffiliation),
        ("ORCID iD", orcid_str.as_str(), GlobalField::AuthorOrcid),
    ];

    let mut left_items = Vec::new();
    for (label, val, field) in author_fields {
        let is_selected = app.selected_global_field == field as usize;
        let prefix = if is_selected { "► " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Reset)
        };

        left_items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{prefix}{label:<15}: "), style),
            Span::styled(if val.is_empty() { "<none>" } else { val }, Style::default().fg(Color::Cyan)),
        ])));
    }

    let left_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 👤 Default Author Requisites ");

    let left_list = List::new(left_items).block(left_block);
    frame.render_widget(left_list, chunks[0]);

    // Right Column: Default Grant & Article Defaults
    let right_fields = [
        ("Grant Funder", app.global_settings.default_grant.funder.as_str(), GlobalField::GrantFunder),
        ("Grant Number", app.global_settings.default_grant.grant_number.as_str(), GlobalField::GrantNumber),
        ("Acknowledgment", app.global_settings.default_grant.acknowledgment.as_str(), GlobalField::GrantAck),
        ("Default Engine", app.global_settings.default_latex_engine.as_str(), GlobalField::Engine),
        ("Default Template", app.global_settings.default_template.as_str(), GlobalField::Template),
    ];

    let mut right_items = Vec::new();
    for (label, val, field) in right_fields {
        let is_selected = app.selected_global_field == field as usize;
        let prefix = if is_selected { "► " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Reset)
        };

        right_items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{prefix}{label:<16}: "), style),
            Span::styled(if val.is_empty() { "<none>" } else { val }, Style::default().fg(Color::Green)),
        ])));
    }

    let right_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(" 📜 Grant & Article Defaults ");

    let right_list = List::new(right_items).block(right_block);
    frame.render_widget(right_list, chunks[1]);
}

fn draw_local_settings(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5),  // Title & Notes
            Constraint::Percentage(50), // Co-authors list
            Constraint::Percentage(50), // Grants list
        ])
        .split(area);

    // 1. Article Title & Notes
    let is_title_sel = app.selected_local_field == LocalField::Title as usize;
    let is_notes_sel = app.selected_local_field == LocalField::Notes as usize;

    let info_text = vec![
        Line::from(vec![
            Span::styled(if is_title_sel { "► Title: " } else { "  Title: " }, if is_title_sel { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default() }),
            Span::styled(if app.local_settings.title.is_empty() { "<empty title>" } else { &app.local_settings.title }, Style::default().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled(if is_notes_sel { "► Notes: " } else { "  Notes: " }, if is_notes_sel { Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD) } else { Style::default() }),
            Span::styled(if app.local_settings.notes.is_empty() { "<no notes>" } else { &app.local_settings.notes }, Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let info_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 📄 Article Particulars ");

    frame.render_widget(Paragraph::new(info_text).block(info_block), chunks[0]);

    // 2. Co-authors List
    let is_ca_sel = app.selected_local_field == LocalField::CoAuthorsList as usize;
    let ca_border_style = if is_ca_sel {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Magenta)
    };

    let coauthor_rows: Vec<Row> = app
        .local_settings
        .co_authors
        .iter()
        .enumerate()
        .map(|(idx, ca)| {
            let style = if is_ca_sel && idx == app.local_coauthor_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Span::styled(&ca.name, style),
                Span::raw(&ca.email),
                Span::raw(&ca.affiliation),
                Span::raw(ca.orcid.as_deref().unwrap_or("")),
            ])
        })
        .collect();

    let ca_table = Table::new(
        coauthor_rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["Co-Author Name", "Email", "Affiliation", "ORCID"])
            .style(Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(ca_border_style)
            .title(" 👥 Co-Authors on this Work (Press 'a' to pick from cache, 'd' to delete) "),
    );

    frame.render_widget(ca_table, chunks[1]);

    // 3. Article Grants List
    let is_gr_sel = app.selected_local_field == LocalField::GrantsList as usize;
    let gr_border_style = if is_gr_sel {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Green)
    };

    let grant_rows: Vec<Row> = app
        .local_settings
        .grants
        .iter()
        .enumerate()
        .map(|(idx, g)| {
            let style = if is_gr_sel && idx == app.local_grant_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Span::styled(&g.funder, style),
                Span::raw(&g.grant_number),
                Span::raw(&g.acknowledgment),
            ])
        })
        .collect();

    let gr_table = Table::new(
        grant_rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ],
    )
    .header(
        Row::new(vec!["Funder", "Grant Number", "Acknowledgment"])
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(gr_border_style)
            .title(" 💰 Grants for this Work (Press 'a' to pick from cache, 'd' to delete) "),
    );

    frame.render_widget(gr_table, chunks[2]);
}

fn draw_coauthor_cache(frame: &mut Frame, app: &App, area: Rect) {
    let rows: Vec<Row> = app
        .cache
        .co_authors
        .iter()
        .enumerate()
        .map(|(idx, ca)| {
            let is_sel = idx == app.cache_coauthor_index;
            let style = if is_sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Span::styled(if is_sel { format!("► {}", ca.name) } else { format!("  {}", ca.name) }, style),
                Span::raw(&ca.email),
                Span::raw(&ca.affiliation),
                Span::raw(ca.orcid.as_deref().unwrap_or("-")),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(30),
            Constraint::Percentage(20),
        ],
    )
    .header(
        Row::new(vec!["Cached Author Name", "Email", "Affiliation", "ORCID"])
            .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
            .title(" 💾 Historical Co-Authors Cache (Press 'u' to use in local project, 'a' to add new, 'd' to delete) "),
    );

    frame.render_widget(table, area);
}

fn draw_grant_cache(frame: &mut Frame, app: &App, area: Rect) {
    let rows: Vec<Row> = app
        .cache
        .grants
        .iter()
        .enumerate()
        .map(|(idx, g)| {
            let is_sel = idx == app.cache_grant_index;
            let style = if is_sel {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            Row::new(vec![
                Span::styled(if is_sel { format!("► {}", g.funder) } else { format!("  {}", g.funder) }, style),
                Span::raw(&g.grant_number),
                Span::raw(&g.acknowledgment),
            ])
        })
        .collect();

    let table = Table::new(
        rows,
        [
            Constraint::Percentage(30),
            Constraint::Percentage(30),
            Constraint::Percentage(40),
        ],
    )
    .header(
        Row::new(vec!["Funder", "Grant Number", "Acknowledgment"])
            .style(Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Green))
            .title(" 💵 Historical Grants Cache (Press 'u' to use in local project, 'a' to add new, 'd' to delete) "),
    );

    frame.render_widget(table, area);
}

fn draw_footer(frame: &mut Frame, app: &App, area: Rect) {
    let dirty_indicator = if app.dirty { " [UNSAVED CHANGES] " } else { "" };

    let footer_text = Paragraph::new(Line::from(vec![
        Span::styled(&app.status_message, Style::default().fg(Color::White)),
        Span::styled(dirty_indicator, Style::default().fg(Color::Red).add_modifier(Modifier::BOLD)),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::DarkGray))
            .title(" Status & Help "),
    );

    frame.render_widget(footer_text, area);
}

fn draw_editing_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(60, 20, frame.area());
    frame.render_widget(Clear, area);

    let popup_block = Block::default()
        .title(" Edit Value (Enter to confirm, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Yellow));

    let input_p = Paragraph::new(app.input_buffer.as_str())
        .style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))
        .block(popup_block);

    frame.render_widget(input_p, area);
}

fn draw_modal_picker(frame: &mut Frame, app: &App) {
    let area = centered_rect(70, 60, frame.area());
    frame.render_widget(Clear, area);

    if app.selected_local_field == LocalField::CoAuthorsList as usize {
        let items: Vec<ListItem> = app
            .cache
            .co_authors
            .iter()
            .enumerate()
            .map(|(idx, ca)| {
                let style = if idx == app.cache_coauthor_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
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
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        frame.render_widget(list, area);
    } else {
        let items: Vec<ListItem> = app
            .cache
            .grants
            .iter()
            .enumerate()
            .map(|(idx, g)| {
                let style = if idx == app.cache_grant_index {
                    Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                ListItem::new(format!("{} (#{})", g.funder, g.grant_number)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .title(" Select Grant from Cache (Enter: Select, 'n': Add New, Esc: Cancel) ")
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .border_style(Style::default().fg(Color::Green)),
            )
            .highlight_style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD));

        frame.render_widget(list, area);
    }
}

fn draw_modal_add_author(frame: &mut Frame, app: &App) {
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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{prefix}{label:<12}: "), style),
            Span::styled(if val.is_empty() { "_" } else { val }, Style::default().fg(Color::Cyan)),
        ]));
    }

    let block = Block::default()
        .title(" Add New Co-Author (Tab to switch field, Enter to save, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Magenta));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

fn draw_modal_add_grant(frame: &mut Frame, app: &App) {
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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };
        lines.push(Line::from(vec![
            Span::styled(format!("{prefix}{label:<15}: "), style),
            Span::styled(if val.is_empty() { "_" } else { val }, Style::default().fg(Color::Green)),
        ]));
    }

    let block = Block::default()
        .title(" Add New Grant Requisites (Tab to switch field, Enter to save, Esc to cancel) ")
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Green));

    frame.render_widget(Paragraph::new(lines).block(block), area);
}

/// Helper function to create a centered Rect for modals/popups.
fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
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
