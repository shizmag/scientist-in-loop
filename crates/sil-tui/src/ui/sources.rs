//! Sources tab and viewing source references modal rendering.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Row, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Table, TableState, Wrap,
    },
};

use super::{centered_rect, references::draw_reference_inspector_card};
use crate::app::{App, InputMode, SourceBadges};

pub(crate) fn draw_sources(frame: &mut Frame, app: &App, area: Rect) {
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

    let unparsed_count = app.sources.iter().filter(|s| !s.parsed).count();

    let (banner_area, list_area) = if !app.sources.is_empty() && unparsed_count > 0 {
        let sc = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Length(4), Constraint::Min(5)])
            .split(chunks[0]);
        (Some(sc[0]), sc[1])
    } else {
        (None, chunks[0])
    };

    if let Some(b_area) = banner_area {
        let banner_text = Paragraph::new(Line::from(vec![Span::styled(
            format!("{unparsed_count} unparsed — [e: Parse selected / Shift+E: Parse all]"),
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]))
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(Color::Yellow))
                .title(" Action Required "),
        )
        .wrap(Wrap { trim: true });
        frame.render_widget(banner_text, b_area);
    }

    // Left column: Sources list
    let avail_w = (chunks[0].width.saturating_sub(4) as usize).max(10);
    let mut items = Vec::new();
    let bib_entries: Vec<sil_core::BibEntryInfo> = app
        .bib_file_entries
        .iter()
        .map(|s| sil_core::extract_bib_entry_info(s))
        .collect();

    if app.sources.is_empty() {
        let empty_msg =
            "No sources found. Drop a PDF/MD in sources/ or Fetch by DOI/URL [a: Add Source]";
        let wrapped = textwrap::wrap(empty_msg, avail_w);
        let lines: Vec<Line> = wrapped
            .into_iter()
            .map(|l| {
                Line::from(Span::styled(
                    l.to_string(),
                    Style::default().fg(Color::DarkGray),
                ))
            })
            .collect();
        items.push(ListItem::new(lines));
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

            let badges =
                SourceBadges::derive(src, &bib_entries, &app.paper_draft_content);

            let status_span = if app.in_flight_parse_ids.contains(&src.id) {
                Span::styled(
                    "[⏳ Parsing...] ",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                )
            } else if badges.parsed {
                Span::styled(
                    format!("{} ", badges.format_badge()),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                )
            } else {
                Span::styled(
                    format!("{} ", badges.format_badge()),
                    Style::default().fg(Color::Yellow),
                )
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

    let list_title = if app.sources.is_empty() {
        " 📚 Source Documents [a: Add Source] ".to_string()
    } else if unparsed_count > 0 {
        format!(" 📚 Sources ({unparsed_count} unparsed) ")
    } else {
        " 📚 Source Documents ('e'/'E': Parse, 'a': Add Link, 'b': Add to Bib, 'r': Rename, 'd': Delete, 'v': Refs, Enter: Read) ".to_string()
    };

    let list_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Cyan))
        .title(list_title);

    let list = List::new(items).block(list_block);
    let mut sources_state = ListState::default();
    if !app.sources.is_empty() {
        sources_state.select(Some(app.selected_source_index));
    }
    frame.render_stateful_widget(list, list_area, &mut sources_state);

    if !app.sources.is_empty() {
        let mut scrollbar_state =
            ScrollbarState::new(app.sources.len()).position(app.selected_source_index);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        frame.render_stateful_widget(scrollbar, list_area, &mut scrollbar_state);
    }

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
    } else if app.sources.is_empty() {
        vec![Line::from(Span::styled(
            "No sources found. Drop a PDF/MD in sources/ or Fetch by DOI/URL [a: Add Source]",
            Style::default().fg(Color::DarkGray),
        ))]
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

pub(crate) fn draw_viewing_source_refs(frame: &mut Frame, app: &App) {
    let modal_area = centered_rect(92, 85, frame.area());
    frame.render_widget(Clear, modal_area);

    let filename = if !app.sources.is_empty() && app.selected_source_index < app.sources.len() {
        &app.sources[app.selected_source_index].filename
    } else {
        "Project"
    };

    let sort_label = match app.ref_sort_key {
        crate::app::RefSortKey::Index => "Index 🔢",
        crate::app::RefSortKey::Year => "Year 📅",
        crate::app::RefSortKey::Source => "Source 📄",
        crate::app::RefSortKey::Venue => "Venue 🏛️",
        crate::app::RefSortKey::Similarity => "Similarity 🎯",
        crate::app::RefSortKey::Title => "Title 📝",
    };

    let filtered = app.filtered_viewing_source_references();
    let total_count = app.selected_source_references.len();
    let filter_count = filtered.len();

    let title_line = if app.input_mode == InputMode::SearchingViewingRefs {
        format!(
            " 📚 References: {filename} ({filter_count}/{total_count}) | Sort: {sort_label} | Search: {}_ ",
            app.viewing_ref_search_query
        )
    } else if !app.viewing_ref_search_query.is_empty() {
        format!(
            " 📚 References: {filename} ({filter_count}/{total_count}) | Sort: {sort_label} | Filter: {} ",
            app.viewing_ref_search_query
        )
    } else {
        format!(" 📚 References: {filename} ({total_count} items) | Sort: {sort_label} ")
    };

    let outer_block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Double)
        .border_style(Style::default().fg(Color::Cyan))
        .title(Span::styled(
            title_line,
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ));

    let inner_area = outer_block.inner(modal_area);
    frame.render_widget(outer_block, modal_area);

    let (table_area, detail_area) = if app.viewing_ref_show_detail {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Percentage(58), Constraint::Percentage(42)])
            .split(inner_area);
        (chunks[0], Some(chunks[1]))
    } else {
        (inner_area, None)
    };

    let rows: Vec<Row> = if filtered.is_empty() {
        vec![Row::new(vec![
            Span::raw("-"),
            Span::styled(
                "No references match criteria.",
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw("-"),
            Span::raw("-"),
            Span::raw("-"),
            Span::raw("-"),
        ])]
    } else {
        filtered
            .iter()
            .enumerate()
            .map(|(i, r)| {
                let is_sel = i == app.selected_viewing_ref_index;
                let style = if is_sel {
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default()
                };
                let prefix = if is_sel { "► " } else { "  " };
                let marked = if app.marked_ref_ids.contains(&r.id) {
                    "[x] "
                } else {
                    "    "
                };
                let marked_style = if app.marked_ref_ids.contains(&r.id) {
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };

                let id_badge = {
                    let mut badges = Vec::new();
                    if r.doi.is_some() {
                        badges.push("DOI");
                    }
                    if r.arxiv_id.is_some() {
                        badges.push("arXiv");
                    }
                    if r.url.is_some() {
                        badges.push("URL");
                    }
                    if badges.is_empty() {
                        String::new()
                    } else {
                        format!("[{}]", badges.join("|"))
                    }
                };

                Row::new(vec![
                    Span::styled(prefix, style),
                    Span::styled(format!("{}[{}]", marked, r.ref_index), marked_style),
                    Span::styled(r.authors.as_deref().unwrap_or("-"), style),
                    Span::styled(r.title.as_deref().unwrap_or(&r.raw_text), style),
                    Span::styled(
                        r.venue.as_deref().unwrap_or("-"),
                        Style::default().fg(Color::Green),
                    ),
                    Span::styled(
                        r.year
                            .map(|y| y.to_string())
                            .unwrap_or_else(|| "-".to_string()),
                        style,
                    ),
                    Span::styled(id_badge, Style::default().fg(Color::LightBlue)),
                ])
                .style(if is_sel {
                    Style::default().bg(Color::Rgb(30, 40, 60))
                } else {
                    Style::default()
                })
            })
            .collect()
    };

    let table = Table::new(
        rows,
        [
            Constraint::Length(10),
            Constraint::Percentage(22),
            Constraint::Percentage(42),
            Constraint::Percentage(16),
            Constraint::Length(6),
            Constraint::Length(15),
        ],
    )
    .header(
        Row::new(vec![
            "#",
            "Authors",
            "Title / Citation Text",
            "Venue",
            "Year",
            "IDs",
        ])
        .style(
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        ),
    )
    .block(
        Block::default()
            .borders(Borders::BOTTOM)
            .border_style(Style::default().fg(Color::DarkGray)),
    );

    let mut state = TableState::default();
    if !filtered.is_empty() {
        state.select(Some(app.selected_viewing_ref_index));
    }
    frame.render_stateful_widget(table, table_area, &mut state);

    if filter_count > 0 {
        let mut scrollbar_state =
            ScrollbarState::new(filter_count).position(app.selected_viewing_ref_index);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        frame.render_stateful_widget(scrollbar, table_area, &mut scrollbar_state);
    }

    if let Some(detail_box_area) = detail_area {
        if let Some(sel_ref) = filtered.get(app.selected_viewing_ref_index) {
            draw_reference_inspector_card(frame, sel_ref, detail_box_area);
        } else {
            let empty_block = Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title(" Reference Details ");
            frame.render_widget(empty_block, detail_box_area);
        }
    }
}
