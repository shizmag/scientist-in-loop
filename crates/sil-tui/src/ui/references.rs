//! References tab and reference inspector card rendering for `sil-tui`.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, Wrap,
    },
};

use crate::app::{App, InputMode};

pub(crate) fn draw_references(frame: &mut Frame, app: &App, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(area);

    // Left pane (references.bib)
    let left_style = if app.active_ref_pane == crate::app::RefPane::LeftBib {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let (cited_bib, total_bib, unmentioned_keys) = if let Some(ref root) = app.project_root {
        let draft_path = root.join("paper_draft.tex");
        let bib_path = root.join("references.bib");
        let bib_opt = if bib_path.is_file() {
            Some(bib_path.as_path())
        } else {
            None
        };
        if let Ok(report) = sil_latex::audit_manuscript(&draft_path, bib_opt) {
            let unmentioned: std::collections::HashSet<_> = report
                .diagnostics
                .iter()
                .filter(|d| d.category == "unmentioned_reference")
                .filter_map(|d| {
                    let msg = &d.message;
                    if let (Some(s), Some(e)) = (msg.find('\''), msg.rfind('\'')) {
                        if s < e {
                            Some(msg[s + 1..e].to_string())
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect();

            let (cited, total) = report.bib_citation_ratio();
            (cited, total, unmentioned)
        } else {
            (
                0,
                app.bib_file_entries.len(),
                std::collections::HashSet::new(),
            )
        }
    } else {
        (
            0,
            app.bib_file_entries.len(),
            std::collections::HashSet::new(),
        )
    };

    let filtered_bib = app.filtered_bib_entries();
    let count_bib = filtered_bib.len();

    let left_title = if app.input_mode == InputMode::SearchingBib {
        format!(
            " references.bib ({cited_bib}/{total_bib} cited) ({count_bib} items) (Search: {}_) ",
            app.bib_search_query
        )
    } else if !app.bib_search_query.is_empty() {
        format!(
            " references.bib ({cited_bib}/{total_bib} cited) ({count_bib} items) (Filter: {}) ",
            app.bib_search_query
        )
    } else {
        format!(" references.bib ({cited_bib}/{total_bib} cited) ")
    };

    let left_width = chunks[0].width.saturating_sub(4) as usize;
    let mut left_items = Vec::new();

    if filtered_bib.is_empty() {
        let empty_msg = if total_bib == 0 {
            "(references.bib is empty)"
        } else {
            "(no search matches in references.bib)"
        };
        left_items.push(ListItem::new(Span::styled(
            empty_msg,
            Style::default().fg(Color::DarkGray),
        )));
    } else {
        for (i, entry) in filtered_bib.iter().enumerate() {
            let is_sel = i == app.selected_bib_index;
            let prefix = if is_sel { "► " } else { "  " };
            let style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            let entry_key = extract_bib_key_from_entry_text(entry);
            let is_cited = entry_key
                .as_ref()
                .is_some_and(|k| !unmentioned_keys.contains(k));
            let status_tag = if is_cited {
                "[✓ cited] "
            } else {
                "[uncited] "
            };
            let status_style = if is_cited {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let mut item_lines = Vec::new();
            let avail_w = left_width.saturating_sub(2).max(10);
            for (line_idx, raw_line) in entry.lines().enumerate() {
                let wrapped = textwrap::wrap(raw_line, avail_w);
                if wrapped.is_empty() {
                    let indent = if line_idx == 0 { prefix } else { "  " };
                    if line_idx == 0 {
                        item_lines.push(Line::from(vec![
                            Span::styled(indent, style),
                            Span::styled(status_tag, status_style),
                        ]));
                    } else {
                        item_lines.push(Line::from(vec![
                            Span::styled(indent, style),
                            Span::styled("", style),
                        ]));
                    }
                } else {
                    for (w_idx, w_sub) in wrapped.iter().enumerate() {
                        let indent = if line_idx == 0 && w_idx == 0 {
                            prefix
                        } else {
                            "  "
                        };
                        if line_idx == 0 && w_idx == 0 {
                            item_lines.push(Line::from(vec![
                                Span::styled(indent, style),
                                Span::styled(status_tag, status_style),
                                Span::styled(w_sub.to_string(), style),
                            ]));
                        } else {
                            item_lines.push(Line::from(vec![
                                Span::styled(indent, style),
                                Span::styled(w_sub.to_string(), style),
                            ]));
                        }
                    }
                }
            }
            left_items.push(ListItem::new(item_lines));
        }
    }

    let left_list = List::new(left_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(left_style)
            .title(left_title),
    );
    let mut left_state = ListState::default();
    if !filtered_bib.is_empty() {
        left_state.select(Some(app.selected_bib_index));
    }
    frame.render_stateful_widget(left_list, chunks[0], &mut left_state);

    if count_bib > 0 {
        let mut scrollbar_state = ScrollbarState::new(count_bib).position(app.selected_bib_index);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        frame.render_stateful_widget(scrollbar, chunks[0], &mut scrollbar_state);
    }

    // Right pane (source_references)
    let right_style = if app.active_ref_pane == crate::app::RefPane::RightSources {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::Cyan)
    };

    let total_refs = app.source_references.len();
    let filtered_refs = app.filtered_source_references();
    let count_refs = filtered_refs.len();

    let right_title = if app.input_mode == InputMode::SearchingRefs {
        format!(
            " Extracted References ({count_refs}/{total_refs}) | Sort: [m]atch/sim [y]ear [v]enue [s]ource [i]ndex [t]itle (Search: {}_) ",
            app.ref_search_query
        )
    } else if !app.ref_search_query.is_empty() {
        format!(
            " Extracted References ({count_refs}/{total_refs}) | Sort: [m]atch/sim [y]ear [v]enue [s]ource [i]ndex [t]itle (Filter: {}) ",
            app.ref_search_query
        )
    } else {
        format!(
            " Extracted References ({total_refs}) | Sort: [m]atch/sim [y]ear [v]enue [s]ource [i]ndex [t]itle "
        )
    };

    let right_width = chunks[1].width.saturating_sub(4) as usize;
    let mut right_items = Vec::new();

    if filtered_refs.is_empty() {
        let empty_msg = if total_refs == 0 {
            "No references extracted. Select a parsed source in Sources tab and press 'v' to view/extract refs."
        } else {
            "(no search matches)"
        };
        let avail_w = right_width.saturating_sub(2).max(10);
        let wrapped = textwrap::wrap(empty_msg, avail_w);
        let lines: Vec<Line> = wrapped
            .into_iter()
            .map(|l| Line::from(Span::styled(l.to_string(), Style::default().fg(Color::DarkGray))))
            .collect();
        right_items.push(ListItem::new(lines));
    } else {
        for (i, entry) in filtered_refs.iter().enumerate() {
            let is_sel = i == app.selected_source_ref_index;
            let style = if is_sel {
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if is_sel { "► " } else { "  " };
            let mark = if app.marked_ref_ids.contains(&entry.id) {
                "[x] "
            } else {
                "[ ] "
            };
            let mark_style = if app.marked_ref_ids.contains(&entry.id) {
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::DarkGray)
            };

            let mut item_lines = Vec::new();

            let mut badges = Vec::new();
            if entry.doi.is_some() {
                badges.push("DOI");
            }
            if entry.arxiv_id.is_some() {
                badges.push("arXiv");
            }
            if entry.url.is_some() {
                badges.push("URL");
            }

            // Title / Header line
            let title_str = entry.title.as_deref().unwrap_or_else(|| &entry.raw_text);
            let mut header_spans = vec![
                Span::styled(prefix, style),
                Span::styled(mark, mark_style),
                Span::styled(
                    format!("Ref #{}: ", entry.ref_index),
                    Style::default().fg(Color::Cyan),
                ),
                Span::styled(title_str, style),
            ];
            if let Some(&sim) = app.draft_ref_similarities.get(&entry.id) {
                header_spans.push(Span::styled(
                    format!(" [{:.2}]", sim),
                    Style::default()
                        .fg(Color::LightMagenta)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            if !badges.is_empty() {
                header_spans.push(Span::styled(
                    format!(" [{}]", badges.join("|")),
                    Style::default()
                        .fg(Color::LightBlue)
                        .add_modifier(Modifier::BOLD),
                ));
            }
            item_lines.push(Line::from(header_spans));

            // Metadata line if structured fields present
            let mut meta_parts = Vec::new();
            if let Some(ref authors) = entry.authors {
                meta_parts.push(format!("Authors: {}", authors));
            }
            if let Some(ref venue) = entry.venue {
                meta_parts.push(format!("Venue: {}", venue));
            }
            if let Some(year) = entry.year {
                meta_parts.push(format!("Year: {}", year));
            }
            if let Some(ref doi) = entry.doi {
                meta_parts.push(format!("DOI: {}", doi));
            }
            if let Some(ref arxiv_id) = entry.arxiv_id {
                meta_parts.push(format!("arXiv: {}", arxiv_id));
            }
            if let Some(ref url) = entry.url {
                meta_parts.push(format!("URL: {}", url));
            }
            if !meta_parts.is_empty() {
                item_lines.push(Line::from(vec![
                    Span::styled("    ", Style::default()),
                    Span::styled(meta_parts.join(" | "), Style::default().fg(Color::Magenta)),
                ]));
            }

            // Formatted raw text wrapped with textwrap
            let avail_w = right_width.saturating_sub(9).max(10);
            let wrapped_raw = textwrap::wrap(&entry.raw_text, avail_w);
            for (w_idx, line) in wrapped_raw.iter().enumerate() {
                let line_prefix = if w_idx == 0 { "    Raw: " } else { "         " };
                item_lines.push(Line::from(vec![
                    Span::styled(line_prefix, Style::default().fg(Color::DarkGray)),
                    Span::styled(line.to_string(), style),
                ]));
            }

            right_items.push(ListItem::new(item_lines));
        }
    }

    let right_list = List::new(right_items).block(
        Block::default()
            .borders(Borders::ALL)
            .border_style(right_style)
            .title(right_title),
    );
    let mut right_state = ListState::default();
    if !filtered_refs.is_empty() {
        right_state.select(Some(app.selected_source_ref_index));
    }
    frame.render_stateful_widget(right_list, chunks[1], &mut right_state);

    if count_refs > 0 {
        let mut scrollbar_state =
            ScrollbarState::new(count_refs).position(app.selected_source_ref_index);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        frame.render_stateful_widget(scrollbar, chunks[1], &mut scrollbar_state);
    }
}

pub(crate) fn draw_reference_inspector_card(
    frame: &mut Frame,
    entry: &sil_core::ReferenceEntry,
    area: Rect,
) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(Color::Green))
        .title(Span::styled(
            format!(" 🔎 Reference Details (Ref #{}) ", entry.ref_index),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        ));

    let inner = block.inner(area);
    frame.render_widget(block, area);

    let chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(55), Constraint::Percentage(45)])
        .split(inner);

    let title_str = entry.title.as_deref().unwrap_or(&entry.raw_text);
    let authors_str = entry.authors.as_deref().unwrap_or("Unknown");
    let venue_str = entry.venue.as_deref().unwrap_or("Unknown");
    let year_str = entry
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "n.d.".to_string());
    let doi_str = entry.doi.as_deref().unwrap_or("None");
    let arxiv_str = entry.arxiv_id.as_deref().unwrap_or("None");
    let url_str = entry.url.as_deref().unwrap_or("None");

    let text_lines = vec![
        Line::from(vec![
            Span::styled(
                "📝 Title: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(title_str, Style::default().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled("👥 Authors: ", Style::default().fg(Color::Magenta)),
            Span::styled(authors_str, Style::default().fg(Color::Reset)),
        ]),
        Line::from(vec![
            Span::styled("🏛️ Venue: ", Style::default().fg(Color::Cyan)),
            Span::styled(venue_str, Style::default().fg(Color::Reset)),
            Span::styled(
                format!("  📅 Year: {year_str}"),
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("🔗 DOI: ", Style::default().fg(Color::LightBlue)),
            Span::styled(
                doi_str,
                Style::default()
                    .fg(Color::LightBlue)
                    .add_modifier(Modifier::UNDERLINED),
            ),
            Span::styled("  🆔 arXiv: ", Style::default().fg(Color::LightYellow)),
            Span::styled(arxiv_str, Style::default().fg(Color::LightYellow)),
        ]),
        Line::from(vec![
            Span::styled("🌐 URL: ", Style::default().fg(Color::LightGreen)),
            Span::styled(url_str, Style::default().fg(Color::LightGreen)),
        ]),
        Line::from(vec![
            Span::styled("📄 Raw: ", Style::default().fg(Color::DarkGray)),
            Span::styled(&entry.raw_text, Style::default().fg(Color::DarkGray)),
        ]),
    ];

    let left_para = Paragraph::new(text_lines).wrap(Wrap { trim: true });
    frame.render_widget(left_para, chunks[0]);

    let bibtex_code = entry.to_bibtex();
    let cite_key = sil_core::slug_cite_key(title_str);

    let mut bib_lines = vec![Line::from(vec![
        Span::styled(
            "⚡ BibTeX Snippet ",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            format!("(\\cite{{{}}})", cite_key),
            Style::default().fg(Color::Yellow),
        ),
    ])];

    for line in bibtex_code.lines() {
        if line.starts_with('@') {
            bib_lines.push(Line::from(Span::styled(
                line,
                Style::default()
                    .fg(Color::Magenta)
                    .add_modifier(Modifier::BOLD),
            )));
        } else if line.contains('=') {
            let parts: Vec<&str> = line.splitn(2, '=').collect();
            bib_lines.push(Line::from(vec![
                Span::styled(parts[0], Style::default().fg(Color::Cyan)),
                Span::styled("=", Style::default().fg(Color::DarkGray)),
                Span::styled(
                    parts.get(1).copied().unwrap_or(""),
                    Style::default().fg(Color::Green),
                ),
            ]));
        } else {
            bib_lines.push(Line::from(Span::styled(
                line,
                Style::default().fg(Color::Reset),
            )));
        }
    }

    let bib_block = Block::default()
        .borders(Borders::LEFT)
        .border_style(Style::default().fg(Color::DarkGray));
    let bib_para = Paragraph::new(bib_lines)
        .block(bib_block)
        .wrap(Wrap { trim: false });
    frame.render_widget(bib_para, chunks[1]);
}

fn extract_bib_key_from_entry_text(entry: &str) -> Option<String> {
    for line in entry.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('@') {
            if let Some(brace) = trimmed.find('{') {
                let rest = &trimmed[brace + 1..];
                if let Some(comma) = rest.find(',') {
                    let key = rest[..comma].trim().to_string();
                    if !key.is_empty() {
                        return Some(key);
                    }
                }
            }
        }
    }
    None
}
