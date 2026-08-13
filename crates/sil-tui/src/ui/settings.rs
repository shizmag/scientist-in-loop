//! Settings view rendering for `sil-tui`.

use ratatui::{
    Frame,
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, BorderType, Borders, List, ListItem, ListState, Scrollbar, ScrollbarOrientation,
        ScrollbarState,
    },
};

use crate::app::{App, DigestField, GlobalField, RagField, SettingItem};

pub(crate) fn draw_settings(frame: &mut Frame, app: &App, area: Rect) {
    let items = app.setting_items();
    let mut list_items = Vec::new();

    let mut current_section = "";

    let num_threads_str = app.global_settings.rag.num_threads.to_string();
    let parent_chunk_str = app.global_settings.rag.parent_chunk_size.to_string();
    let child_chunk_str = app.global_settings.rag.child_chunk_size.to_string();
    let digest_refresh_str = app.global_settings.digest_refresh_hours.to_string();

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
            SettingItem::Digest(_) => "Digest Settings",
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
                "Digest Settings" => "📰",
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
                    RagField::XbergCacheDir => (
                        "Xberg Cache Dir",
                        app.global_settings.rag.xberg_model_cache_dir.as_str(),
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
            SettingItem::Digest(f) => {
                let (label, val) = match f {
                    DigestField::GlobalQuery => (
                        "Global Digest Query",
                        app.global_settings.digest_query.as_str(),
                    ),
                    DigestField::RefreshHours => {
                        ("Refresh Interval (Hours)", digest_refresh_str.as_str())
                    }
                    DigestField::LocalQuery => (
                        "Local Digest Query",
                        app.local_settings.digest_query.as_str(),
                    ),
                };
                Line::from(vec![
                    Span::styled(format!("{prefix}{label:<24}: "), style),
                    Span::styled(
                        if val.is_empty() { "<none>" } else { val },
                        Style::default().fg(Color::Yellow),
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
    let mut settings_state = ListState::default();
    if !app.setting_items().is_empty() {
        settings_state.select(Some(app.selected_setting_index));
    }
    frame.render_stateful_widget(list, area, &mut settings_state);

    let total_settings = app.setting_items().len();
    if total_settings > 0 {
        let mut scrollbar_state =
            ScrollbarState::new(total_settings).position(app.selected_setting_index);
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("▲"))
            .end_symbol(Some("▼"))
            .track_symbol(Some("│"))
            .thumb_symbol("█");
        frame.render_stateful_widget(scrollbar, area, &mut scrollbar_state);
    }
}
