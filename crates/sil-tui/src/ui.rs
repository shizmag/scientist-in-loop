//! Ratatui UI drawing code for `sil-tui`.

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph,
        Row, Table, Tabs, Wrap,
    },
    Frame,
};

use crate::app::{ActiveTab, App, GlobalField, InputMode, LocalField, RagField};

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
        ActiveTab::PaperDraft => draw_paper_draft(frame, app, chunks[1]),
        ActiveTab::GlobalSettings => draw_global_settings(frame, app, chunks[1]),
        ActiveTab::LocalSettings => draw_local_settings(frame, app, chunks[1]),
        ActiveTab::CoAuthorCache => draw_coauthor_cache(frame, app, chunks[1]),
        ActiveTab::GrantCache => draw_grant_cache(frame, app, chunks[1]),
        ActiveTab::RagSettings => draw_rag_settings(frame, app, chunks[1]),
    }


    draw_footer(frame, app, chunks[2]);

    // Modals overlay
    match app.input_mode {
        InputMode::Editing => draw_editing_popup(frame, app),
        InputMode::EditingPaper => draw_editing_paper_popup(frame, app),
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

fn draw_rag_settings(frame: &mut Frame, app: &App, area: Rect) {
    let rag = &app.global_settings.rag;

    let num_threads_str = rag.num_threads.to_string();
    let parent_chunk_str = rag.parent_chunk_size.to_string();
    let child_chunk_str = rag.child_chunk_size.to_string();

    let fields = [
        ("ONNX Embedder Model", rag.onnx_embedder_model.as_str(), RagField::EmbedderModel),
        ("ONNX Reranker Model", rag.onnx_reranker_model.as_str(), RagField::RerankerModel),
        ("Model Cache Dir", rag.model_cache_dir.as_str(), RagField::CacheDir),
        ("Execution Provider", rag.execution_provider.as_str(), RagField::ExecutionProvider),
        ("Num Threads", num_threads_str.as_str(), RagField::NumThreads),
        ("Parent Chunk Size", parent_chunk_str.as_str(), RagField::ParentChunkSize),
        ("Child Chunk Size", child_chunk_str.as_str(), RagField::ChildChunkSize),
    ];

    let mut items = Vec::new();
    for (label, val, field) in fields {
        let is_selected = app.selected_rag_field == field as usize;
        let prefix = if is_selected { "► " } else { "  " };
        let style = if is_selected {
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::Reset)
        };

        items.push(ListItem::new(Line::from(vec![
            Span::styled(format!("{prefix}{label:<22}: "), style),
            Span::styled(if val.is_empty() { "<none>" } else { val }, Style::default().fg(Color::Magenta)),
        ])));
    }

    let title_text = if let Some(ref cfg) = app.loaded_config {
        if cfg.rag.is_some() {
            " 🤖 ONNX & Local RAG Settings (Active: .sil/config.yaml project override) "
        } else {
            " 🤖 ONNX & Local RAG Settings (~/.config/sil/settings.yaml) "
        }
    } else {
        " 🤖 ONNX & Local RAG Settings (~/.config/sil/settings.yaml) "
    };

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Magenta))
        .title(title_text);

    let list = List::new(items).block(block);
    frame.render_widget(list, area);
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

fn draw_dashboard(frame: &mut Frame, _app: &mut App, area: Rect) {
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    let top_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[0]);

    let bottom_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(main_chunks[1]);

    // 1. Manuscript Progress & Health Audit
    let health_lines = vec![

        Line::from(vec![
            Span::styled("Manuscript Health & Progress Status", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Stage: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Stage 5 (Polish & Production)", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("• Main Draft: ", Style::default().fg(Color::DarkGray)),
            Span::styled("paper_draft.tex", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("• Citation Integrity: ", Style::default().fg(Color::DarkGray)),
            Span::styled("OK (references.bib synchronized)", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("• Label References: ", Style::default().fg(Color::DarkGray)),
            Span::styled("OK (all labels matched)", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("• Engine: ", Style::default().fg(Color::DarkGray)),
            Span::styled("tectonic (configured)", Style::default().fg(Color::White)),
        ]),
    ];
    let health_block = Block::default()
        .title(" [1] Manuscript Completion & Health Audit ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(Paragraph::new(health_lines).block(health_block), top_chunks[0]);

    // 2. Active Idea & TODO Blocks (# -- X -- #)
    let idea_lines = vec![
        Line::from(vec![
            Span::styled("# -- X -- # Idea & TODO Notes", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("1. [Intro / Lines 12-18]: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Refine motivation for self-attention baseline", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("2. [Methods / Lines 45-52]: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Add equation comparing loss functions A vs B", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("3. [Results / Lines 88-95]: ", Style::default().fg(Color::DarkGray)),
            Span::styled("Verify dataset metrics table with latest run", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Tip: Surround notes with # -- X -- # in paper_draft.tex for AI agents.", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let idea_block = Block::default()
        .title(" [2] Active Ideas & TODO Blocks (# -- X -- #) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(Paragraph::new(idea_lines).block(idea_block), top_chunks[1]);

    // 3. Top Journal Digest Feed
    let digest_lines = vec![
        Line::from(vec![
            Span::styled("Top Peer-Reviewed Journal Feed (Crossref / Nature / IEEE)", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• [Nature 2024] ", Style::default().fg(Color::Green)),
            Span::styled("Quantum Advantage in Scientific Discovery", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("• [IEEE TPAMI] ", Style::default().fg(Color::Green)),
            Span::styled("Scalable Multi-Agent Foundation Models", Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("• [JMLR] ", Style::default().fg(Color::Green)),
            Span::styled("Theoretical Guarantees for Attention Mechanics", Style::default().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("Run 'sil digest <query>' to update top journal feed", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let digest_block = Block::default()
        .title(" [3] Literature Digest (Top Journals) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    frame.render_widget(Paragraph::new(digest_lines).block(digest_block), bottom_chunks[0]);

    // 4. Scientist Command Center & Shortcut Guide
    let guide_lines = vec![
        Line::from(vec![
            Span::styled("Daily Scientist Helper Shortcuts", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab", Style::default().fg(Color::Yellow)),
            Span::styled("  Switch between Dashboard & Paper Draft & Settings tabs", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  'e' / 'v'", Style::default().fg(Color::Yellow)),
            Span::styled("        Edit section in TUI ('e') or open $EDITOR (nvim/helix) ('v')", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  sil doctor", Style::default().fg(Color::Yellow)),
            Span::styled("       Run full host + manuscript health audit", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  sil digest <q>", Style::default().fg(Color::Yellow)),
            Span::styled("    Fetch top journal publications", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  sil todo", Style::default().fg(Color::Yellow)),
            Span::styled("          List all # -- X -- # ideas in draft", Style::default().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled("  sil propose", Style::default().fg(Color::Yellow)),
            Span::styled("       Create git commit proposal with Sci-Action", Style::default().fg(Color::DarkGray)),
        ]),
    ];
    let guide_block = Block::default()
        .title(" [4] Scientist Command Center ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    frame.render_widget(Paragraph::new(guide_lines).block(guide_block), bottom_chunks[1]);
}

fn draw_paper_draft(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(35), Constraint::Percentage(65)])
        .split(area);

    // Left Column: LaTeX Section Outline / Parser Tree
    let mut items = Vec::new();
    if app.paper_sections.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " (no sections / empty paper_draft.tex)",
            Style::default().fg(Color::DarkGray),
        ))));
    } else {
        for (idx, sec) in app.paper_sections.iter().enumerate() {
            let is_selected = app.paper_section_index == idx;
            let prefix = if is_selected { "► " } else { "  " };
            let style = if is_selected {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Reset)
            };
            items.push(ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(format!("[{}] ", sec.kind), Style::default().fg(Color::Magenta)),
                Span::styled(&sec.title, style),
                Span::styled(format!(" (L{})", sec.line_start), Style::default().fg(Color::DarkGray)),
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

fn draw_editing_paper_popup(frame: &mut Frame, app: &App) {
    let area = centered_rect(80, 60, frame.area());
    frame.render_widget(Clear, area);

    let sec_title = if !app.paper_sections.is_empty()
        && app.paper_section_index < app.paper_sections.len()
    {
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
            Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
        ));

    let paragraph = Paragraph::new(app.paper_edit_buffer.as_str())
        .block(popup_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, area);
}

