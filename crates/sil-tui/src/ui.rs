//! Ratatui UI drawing code for `sil-tui`.

use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Row, Table, Tabs, Wrap,
    },
};

use crate::app::{ActiveTab, App, GlobalField, InputMode, RagField, SettingItem};

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
        ActiveTab::Sources => draw_sources(frame, app, chunks[1]),
        ActiveTab::References => draw_references(frame, app, chunks[1]),
        ActiveTab::PaperDraft => draw_paper_draft(frame, app, chunks[1]),
        ActiveTab::Settings => draw_settings(frame, app, chunks[1]),
    }

    draw_footer(frame, app, chunks[2]);

    // Modals overlay
    match app.input_mode {
        InputMode::Editing => draw_editing_popup(frame, app),
        InputMode::EditingPaper => draw_editing_paper_popup(frame, app),
        InputMode::ModalPicker => draw_modal_picker(frame, app),
        InputMode::ModalAddAuthor => draw_modal_add_author(frame, app),
        InputMode::ModalAddGrant => draw_modal_add_grant(frame, app),
        InputMode::ModalAddSourceLink => draw_modal_add_source_link(frame, app),
        InputMode::ModalRenameSource => draw_modal_rename_source(frame, app),
        InputMode::ConfirmDeleteSource => draw_confirm_delete_source(frame, app),
        InputMode::ViewingSourceRefs => draw_viewing_source_refs(frame, app),
        InputMode::SearchingRefs | InputMode::ReadingSourceMd | InputMode::Normal => {}
    }
}

fn draw_references(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left pane (references.bib)
    let left_style = if app.active_ref_pane == crate::app::RefPane::LeftBib {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    let mut left_items = Vec::new();
    if app.bib_file_entries.is_empty() {
        left_items.push(ListItem::new(Span::styled("(references.bib is empty)", Style::default().fg(Color::DarkGray))));
    } else {
        for (i, entry) in app.bib_file_entries.iter().enumerate() {
            let prefix = if i == app.selected_bib_index { "► " } else { "  " };
            let style = if i == app.selected_bib_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            left_items.push(ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(entry.replace("\n", " "), style),
            ])));
        }
    }
    let left_list = List::new(left_items)
        .block(Block::default().borders(Borders::ALL).border_style(left_style).title(" references.bib "));
    frame.render_widget(left_list, chunks[0]);

    // Right pane (source_references)
    let right_style = if app.active_ref_pane == crate::app::RefPane::RightSources {
        Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };
    
    let mut right_items = Vec::new();
    let filtered_refs: Vec<_> = if app.ref_search_query.is_empty() {
        app.source_references.iter().collect()
    } else {
        app.source_references.iter().filter(|r| r.raw_text.to_lowercase().contains(&app.ref_search_query.to_lowercase())).collect()
    };
    
    if filtered_refs.is_empty() {
        right_items.push(ListItem::new(Span::styled(
            if app.source_references.is_empty() { "(no project references found)" } else { "(no search matches)" },
            Style::default().fg(Color::DarkGray)
        )));
    } else {
        for (i, entry) in filtered_refs.iter().enumerate() {
            let prefix = if i == app.selected_source_ref_index { "► " } else { "  " };
            let style = if i == app.selected_source_ref_index {
                Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let mark = if app.marked_ref_ids.contains(&entry.id) { "[x] " } else { "[ ] " };
            
            right_items.push(ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                Span::styled(mark, Style::default().fg(Color::Green)),
                Span::styled(&entry.raw_text, style),
            ])));
        }
    }
    
    let search_title = if app.input_mode == InputMode::SearchingRefs {
        format!(" Extracted References (Search: {}_) ", app.ref_search_query)
    } else if !app.ref_search_query.is_empty() {
        format!(" Extracted References (Filter: {}) ", app.ref_search_query)
    } else {
        " Extracted References ".to_string()
    };
    
    let right_list = List::new(right_items)
        .block(Block::default().borders(Borders::ALL).border_style(right_style).title(search_title));
    frame.render_widget(right_list, chunks[1]);
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

fn draw_sources(frame: &mut Frame, app: &App, area: Rect) {
    if app.input_mode == InputMode::ReadingSourceMd {
        let filename = if !app.sources.is_empty() && app.selected_source_index < app.sources.len() {
            &app.sources[app.selected_source_index].filename
        } else {
            "Markdown Document"
        };
        let content_text = app
            .reading_md_content
            .as_deref()
            .unwrap_or("No markdown content loaded.");
        let block = Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                format!(" 📖 Reading Markdown: {filename} (Press Esc / 'q' to exit, j/k/PgUp/PgDn to scroll) "),
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD),
            ));
        let paragraph = Paragraph::new(content_text)
            .block(block)
            .wrap(Wrap { trim: false })
            .scroll((app.source_scroll_offset as u16, 0));
        frame.render_widget(paragraph, area);
        return;
    }

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(40), Constraint::Percentage(60)])
        .split(area);

    // Left column: Sources list
    let mut items = Vec::new();
    if app.sources.is_empty() {
        items.push(ListItem::new(Line::from(Span::styled(
            " (no sources found — press 'a' to add via link) ",
            Style::default().fg(Color::DarkGray),
        ))));
    } else {
        for (idx, src) in app.sources.iter().enumerate() {
            let is_sel = idx == app.selected_source_index;
            let prefix = if is_sel { "► " } else { "  " };
            let style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let status_span = if src.parsed {
                Span::styled(
                    "[✓ Parsed] ",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled("[Unparsed] ", Style::default().fg(Color::DarkGray))
            };

            let kind_span = Span::styled(
                format!("[{}] ", src.kind),
                Style::default().fg(Color::Magenta),
            );
            let name_span = Span::styled(&src.filename, style);

            items.push(ListItem::new(Line::from(vec![
                Span::styled(prefix, style),
                status_span,
                kind_span,
                name_span,
            ])));
        }
    }

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" 📚 Source Documents ('a': Add Link, 'r': Rename, 'd': Delete, 'v': Refs, Enter: Read) ");

    let list = List::new(items).block(list_block);
    frame.render_widget(list, chunks[0]);

    // Right column: Selected Source Metadata & Stats
    let detail_lines = if !app.sources.is_empty() && app.selected_source_index < app.sources.len() {
        let src = &app.sources[app.selected_source_index];
        let title_str = src.title.as_deref().unwrap_or("<Untitled>");
        let authors_str = src.authors.as_deref().unwrap_or("-");
        let venue_str = src.venue.as_deref().unwrap_or("-");
        let year_str = src
            .year
            .map(|y| y.to_string())
            .unwrap_or_else(|| "-".to_string());
        let doi_str = src.doi.as_deref().unwrap_or("-");
        let abstract_str = src.abstract_text.as_deref().unwrap_or("-");

        let ref_count = if let Some(ref refs_text) = src.references_text {
            refs_text.lines().filter(|l| !l.trim().is_empty()).count()
        } else {
            0
        };
        let word_count = abstract_str.split_whitespace().count();

        vec![
            Line::from(vec![
                Span::styled(
                    "Title: ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::styled(title_str, Style::default().fg(Color::Yellow)),
            ]),
            Line::from(vec![
                Span::styled("Filename: ", Style::default().fg(Color::Cyan)),
                Span::raw(&src.filename),
            ]),
            Line::from(vec![
                Span::styled("Path: ", Style::default().fg(Color::Cyan)),
                Span::raw(src.path.as_str()),
            ]),
            Line::from(vec![
                Span::styled("Format/Kind: ", Style::default().fg(Color::Cyan)),
                Span::styled(src.kind.to_string(), Style::default().fg(Color::Magenta)),
                Span::raw("  |  Status: "),
                if src.parsed {
                    Span::styled("[✓ Parsed]", Style::default().fg(Color::Green))
                } else {
                    Span::styled("On Disk / Unparsed", Style::default().fg(Color::DarkGray))
                },
            ]),
            Line::from(vec![
                Span::styled("Authors: ", Style::default().fg(Color::Cyan)),
                Span::raw(authors_str),
            ]),
            Line::from(vec![
                Span::styled("Venue/Year: ", Style::default().fg(Color::Cyan)),
                Span::raw(format!("{venue_str} ({year_str})")),
            ]),
            Line::from(vec![
                Span::styled("DOI: ", Style::default().fg(Color::Cyan)),
                Span::raw(doi_str),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "📊 Document Statistics:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(vec![
                Span::styled("• Abstract Word Count: ", Style::default().fg(Color::Cyan)),
                Span::raw(word_count.to_string()),
            ]),
            Line::from(vec![
                Span::styled(
                    "• Extracted References Count: ",
                    Style::default().fg(Color::Cyan),
                ),
                Span::raw(ref_count.to_string()),
            ]),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Abstract Preview:",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(Span::styled(
                abstract_str,
                Style::default().fg(Color::Reset),
            )),
        ]
    } else {
        vec![Line::from(Span::styled(
            "Select a source to view metadata & statistics.",
            Style::default().fg(Color::DarkGray),
        ))]
    };

    let detail_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(" 📄 Source Details & Statistics ");

    let paragraph = Paragraph::new(detail_lines)
        .block(detail_block)
        .wrap(Wrap { trim: false });

    frame.render_widget(paragraph, chunks[1]);
}

fn draw_settings(frame: &mut Frame, app: &App, area: Rect) {
    let items = app.setting_items();
    let mut list_items = Vec::new();

    let mut current_section = "";

    let num_threads_str = app.global_settings.rag.num_threads.to_string();
    let parent_chunk_str = app.global_settings.rag.parent_chunk_size.to_string();
    let child_chunk_str = app.global_settings.rag.child_chunk_size.to_string();

    for (flat_idx, item) in items.iter().enumerate() {
        let is_sel = app.selected_setting_index == flat_idx;
        let prefix = if is_sel { "► " } else { "  " };
        let style = if is_sel {
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD)
        } else {
            Style::default()
        };

        let section_name = match item {
            SettingItem::Global(_) => "Global Settings",
            SettingItem::Rag(_) => "RAG Settings",
            SettingItem::CacheCoAuthor(_)
            | SettingItem::CacheCoAuthorEmpty
            | SettingItem::CacheGrant(_)
            | SettingItem::CacheGrantEmpty => "Co-Author & Grant Caches",
            SettingItem::LocalTitle
            | SettingItem::LocalNotes
            | SettingItem::LocalCoAuthor(_)
            | SettingItem::LocalCoAuthorEmpty
            | SettingItem::LocalGrant(_)
            | SettingItem::LocalGrantEmpty => "Local Project Settings",
        };

        if section_name != current_section {
            current_section = section_name;
            let icon = match section_name {
                "Global Settings" => "👤",
                "RAG Settings" => "🤖",
                "Co-Author & Grant Caches" => "💾",
                _ => "📄",
            };
            list_items.push(ListItem::new(Line::from(vec![Span::styled(
                format!(
                    "━━━ {icon} {section_name} ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
                ),
                Style::default()
                    .fg(Color::Blue)
                    .add_modifier(Modifier::BOLD),
            )])));
        }

        let line = match item {
            SettingItem::Global(f) => {
                let (label, val) = match f {
                    GlobalField::AuthorName => {
                        ("Author Name", app.global_settings.author.name.as_str())
                    }
                    GlobalField::AuthorEmail => {
                        ("Author Email", app.global_settings.author.email.as_str())
                    }
                    GlobalField::AuthorAffiliation => (
                        "Author Affiliation",
                        app.global_settings.author.affiliation.as_str(),
                    ),
                    GlobalField::AuthorOrcid => (
                        "Author ORCID",
                        app.global_settings.author.orcid.as_deref().unwrap_or(""),
                    ),
                    GlobalField::GrantFunder => (
                        "Default Grant Funder",
                        app.global_settings.default_grant.funder.as_str(),
                    ),
                    GlobalField::GrantNumber => (
                        "Default Grant Number",
                        app.global_settings.default_grant.grant_number.as_str(),
                    ),
                    GlobalField::GrantAck => (
                        "Default Grant Ack",
                        app.global_settings.default_grant.acknowledgment.as_str(),
                    ),
                    GlobalField::Engine => (
                        "Default LaTeX Engine",
                        app.global_settings.default_latex_engine.as_str(),
                    ),
                    GlobalField::Template => (
                        "Default Template",
                        app.global_settings.default_template.as_str(),
                    ),
                };
                Line::from(vec![
                    Span::styled(format!("{prefix}{label:<24}: "), style),
                    Span::styled(
                        if val.is_empty() { "<none>" } else { val },
                        Style::default().fg(Color::Cyan),
                    ),
                ])
            }
            SettingItem::Rag(f) => {
                let (label, val) = match f {
                    RagField::EmbedderPath => (
                        "ONNX Embedder Path/Dir",
                        app.global_settings
                            .rag
                            .onnx_embedder_path
                            .as_ref()
                            .map(|p| p.as_str())
                            .unwrap_or(""),
                    ),
                    RagField::RerankerPath => (
                        "ONNX Reranker Path/Dir",
                        app.global_settings
                            .rag
                            .onnx_reranker_path
                            .as_ref()
                            .map(|p| p.as_str())
                            .unwrap_or(""),
                    ),
                    RagField::ModelsDir => (
                        "Custom ONNX Models Dir",
                        app.global_settings
                            .rag
                            .onnx_models_dir
                            .as_ref()
                            .map(|p| p.as_str())
                            .unwrap_or(""),
                    ),
                    RagField::CacheDir => (
                        "Model Cache Dir",
                        app.global_settings.rag.model_cache_dir.as_str(),
                    ),
                    RagField::ExecutionProvider => (
                        "Execution Provider",
                        app.global_settings.rag.execution_provider.as_str(),
                    ),
                    RagField::NumThreads => ("Num Threads", num_threads_str.as_str()),
                    RagField::ParentChunkSize => ("Parent Chunk Size", parent_chunk_str.as_str()),
                    RagField::ChildChunkSize => ("Child Chunk Size", child_chunk_str.as_str()),
                };
                Line::from(vec![
                    Span::styled(format!("{prefix}{label:<24}: "), style),
                    Span::styled(
                        if val.is_empty() { "<none>" } else { val },
                        Style::default().fg(Color::Magenta),
                    ),
                ])
            }
            SettingItem::CacheCoAuthor(idx) => {
                let ca = &app.cache.co_authors[*idx];
                Line::from(vec![
                    Span::styled(format!("{prefix}Cached Co-Author #{}: ", idx + 1), style),
                    Span::styled(
                        format!("{} <{}> ({})", ca.name, ca.email, ca.affiliation),
                        Style::default().fg(Color::Cyan),
                    ),
                ])
            }
            SettingItem::CacheCoAuthorEmpty => Line::from(vec![
                Span::styled(format!("{prefix}Cached Co-Authors: "), style),
                Span::styled(
                    "(no cached co-authors — press 'a' to add)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            SettingItem::CacheGrant(idx) => {
                let g = &app.cache.grants[*idx];
                Line::from(vec![
                    Span::styled(format!("{prefix}Cached Grant #{}: ", idx + 1), style),
                    Span::styled(
                        format!("{} (#{})", g.funder, g.grant_number),
                        Style::default().fg(Color::Green),
                    ),
                ])
            }
            SettingItem::CacheGrantEmpty => Line::from(vec![
                Span::styled(format!("{prefix}Cached Grants: "), style),
                Span::styled(
                    "(no cached grants — press 'a' to add)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            SettingItem::LocalTitle => Line::from(vec![
                Span::styled(format!("{prefix}Project Title: "), style),
                Span::styled(
                    if app.local_settings.title.is_empty() {
                        "<empty title>"
                    } else {
                        &app.local_settings.title
                    },
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            SettingItem::LocalNotes => Line::from(vec![
                Span::styled(format!("{prefix}Project Notes: "), style),
                Span::styled(
                    if app.local_settings.notes.is_empty() {
                        "<no notes>"
                    } else {
                        &app.local_settings.notes
                    },
                    Style::default().fg(Color::Reset),
                ),
            ]),
            SettingItem::LocalCoAuthor(idx) => {
                let ca = &app.local_settings.co_authors[*idx];
                Line::from(vec![
                    Span::styled(format!("{prefix}Local Co-Author #{}: ", idx + 1), style),
                    Span::styled(
                        format!("{} <{}>", ca.name, ca.email),
                        Style::default().fg(Color::Magenta),
                    ),
                ])
            }
            SettingItem::LocalCoAuthorEmpty => Line::from(vec![
                Span::styled(format!("{prefix}Local Co-Authors: "), style),
                Span::styled(
                    "(none — press 'a' to pick from cache)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
            SettingItem::LocalGrant(idx) => {
                let g = &app.local_settings.grants[*idx];
                Line::from(vec![
                    Span::styled(format!("{prefix}Local Grant #{}: ", idx + 1), style),
                    Span::styled(
                        format!("{} (#{})", g.funder, g.grant_number),
                        Style::default().fg(Color::Green),
                    ),
                ])
            }
            SettingItem::LocalGrantEmpty => Line::from(vec![
                Span::styled(format!("{prefix}Local Grants: "), style),
                Span::styled(
                    "(none — press 'a' to pick from cache)",
                    Style::default().fg(Color::DarkGray),
                ),
            ]),
        };

        list_items.push(ListItem::new(line));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(" ⚙️ Unified Settings (Global | RAG | Caches | Local Project) — Enter/'e': Edit, 'a': Add, 'd': Delete, 'u': Use Cache ");

    let list = List::new(list_items).block(block);
    frame.render_widget(list, area);
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
    {
        Style::default()
            .fg(Color::LightRed)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    };

    let footer_text = Paragraph::new(Line::from(vec![
        Span::styled(&app.status_message, msg_style),
        Span::styled(
            dirty_indicator,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ),
    ]))
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .border_style(Style::default().fg(Color::Cyan))
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
        .style(
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )
        .block(popup_block);

    frame.render_widget(input_p, area);
}

fn draw_modal_picker(frame: &mut Frame, app: &App) {
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

fn draw_modal_add_source_link(frame: &mut Frame, app: &App) {
    let area = centered_rect(65, 25, frame.area());
    frame.render_widget(Clear, area);

    let block = Block::default()
        .title(" Add Source via Link / DOI / arXiv (Enter to fetch, Esc to cancel) ")
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

fn draw_modal_rename_source(frame: &mut Frame, app: &App) {
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

fn draw_confirm_delete_source(frame: &mut Frame, app: &App) {
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

fn draw_viewing_source_refs(frame: &mut Frame, app: &App) {
    let area = centered_rect(88, 75, frame.area());
    frame.render_widget(Clear, area);

    let filename = if !app.sources.is_empty() && app.selected_source_index < app.sources.len() {
        &app.sources[app.selected_source_index].filename
    } else {
        "Project"
    };

    let sort_label = match app.ref_sort_key {
        crate::app::RefSortKey::Index => "Index",
        crate::app::RefSortKey::Year => "Year ⬇",
        crate::app::RefSortKey::Source => "Source",
        crate::app::RefSortKey::Venue => "Journal/Conf",
    };

    let rows: Vec<Row> = if app.selected_source_references.is_empty() {
        vec![Row::new(vec![
            "-",
            "No extracted references found.",
            "-",
            "-",
            "-",
        ])]
    } else {
        app.selected_source_references
            .iter()
            .map(|r| {
                Row::new(vec![
                    Span::styled(
                        format!("[{}]", r.ref_index),
                        Style::default().fg(Color::Cyan),
                    ),
                    Span::raw(r.authors.as_deref().unwrap_or("-")),
                    Span::raw(r.title.as_deref().unwrap_or(&r.raw_text)),
                    Span::styled(
                        r.venue.as_deref().unwrap_or("-"),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw(
                        r.year
                            .map(|y| y.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                    ),
                ])
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(6),
            Constraint::Percentage(25),
            Constraint::Percentage(44),
            Constraint::Percentage(17),
            Constraint::Percentage(8),
        ],
    )
    .header(
        Row::new(vec!["#", "Authors", "Title / Citation Text", "Journal / Conference", "Year"]).style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Double)
            .border_style(Style::default().fg(Color::Cyan))
            .title(Span::styled(
                format!(" 📚 References ({filename}) | Sort: {sort_label} (y: Year, s: Source, v: Venue, i: Index | Esc: Close) "),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )),
    );

    frame.render_widget(table, area);
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
        Line::from(vec![Span::styled(
            "Manuscript Health & Progress Status",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• Stage: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "Stage 5 (Polish & Production)",
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Main Draft: ", Style::default().fg(Color::Cyan)),
            Span::styled("paper_draft.tex", Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("• Citation Integrity: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                "OK (references.bib synchronized)",
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Label References: ", Style::default().fg(Color::Cyan)),
            Span::styled("OK (all labels matched)", Style::default().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled("• Engine: ", Style::default().fg(Color::Cyan)),
            Span::styled("tectonic (configured)", Style::default().fg(Color::Reset)),
        ]),
    ];
    let health_block = Block::default()
        .title(" [1] Manuscript Completion & Health Audit ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Cyan));
    frame.render_widget(
        Paragraph::new(health_lines).block(health_block),
        top_chunks[0],
    );

    // 2. Active Idea & TODO Blocks (# -- X -- #)
    let idea_lines = vec![
        Line::from(vec![Span::styled(
            "# -- X -- # Idea & TODO Notes",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "1. [Intro / Lines 12-18]: ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Refine motivation for self-attention baseline",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "2. [Methods / Lines 45-52]: ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Add equation comparing loss functions A vs B",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled(
                "3. [Results / Lines 88-95]: ",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "Verify dataset metrics table with latest run",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Tip: Surround notes with # -- X -- # in paper_draft.tex for AI agents.",
            Style::default().fg(Color::Reset),
        )]),
    ];
    let idea_block = Block::default()
        .title(" [2] Active Ideas & TODO Blocks (# -- X -- #) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(Paragraph::new(idea_lines).block(idea_block), top_chunks[1]);

    // 3. Top Journal Digest Feed
    let digest_lines = vec![
        Line::from(vec![Span::styled(
            "Top Peer-Reviewed Journal Feed (Crossref / Nature / IEEE)",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("• [Nature 2024] ", Style::default().fg(Color::Green)),
            Span::styled(
                "Quantum Advantage in Scientific Discovery",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("• [IEEE TPAMI] ", Style::default().fg(Color::Green)),
            Span::styled(
                "Scalable Multi-Agent Foundation Models",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("• [JMLR] ", Style::default().fg(Color::Green)),
            Span::styled(
                "Theoretical Guarantees for Attention Mechanics",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(""),
        Line::from(vec![Span::styled(
            "Run 'sil digest <query>' to update top journal feed",
            Style::default().fg(Color::Reset),
        )]),
    ];
    let digest_block = Block::default()
        .title(" [3] Literature Digest (Top Journals) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    frame.render_widget(
        Paragraph::new(digest_lines).block(digest_block),
        bottom_chunks[0],
    );

    // 4. Scientist Command Center & Shortcut Guide
    let guide_lines = vec![
        Line::from(vec![Span::styled(
            "Daily Scientist Helper Shortcuts",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled("  Tab / Shift+Tab", Style::default().fg(Color::Yellow)),
            Span::styled(
                "  Switch between Dashboard, Paper Draft, Sources, and Settings",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  'e' / 'v'", Style::default().fg(Color::Yellow)),
            Span::styled(
                "        Edit section in TUI ('e') or open $EDITOR (nvim/helix) ('v')",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  sil doctor", Style::default().fg(Color::Yellow)),
            Span::styled(
                "       Run full host + manuscript health audit",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  sil digest <q>", Style::default().fg(Color::Yellow)),
            Span::styled(
                "    Fetch top journal publications",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  sil todo", Style::default().fg(Color::Yellow)),
            Span::styled(
                "          List all # -- X -- # ideas in draft",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  sil propose", Style::default().fg(Color::Yellow)),
            Span::styled(
                "       Create git commit proposal with Sci-Action",
                Style::default().fg(Color::Reset),
            ),
        ]),
    ];
    let guide_block = Block::default()
        .title(" [4] Scientist Command Center ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Green));
    frame.render_widget(
        Paragraph::new(guide_lines).block(guide_block),
        bottom_chunks[1],
    );
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

fn draw_editing_paper_popup(frame: &mut Frame, app: &App) {
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
