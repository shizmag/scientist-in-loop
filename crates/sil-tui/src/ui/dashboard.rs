//! Dashboard view rendering for `sil-tui`.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Wrap},
};
use sil_core::JournalPublication;

use crate::app::App;

/// Parsed idea row stored in `DashboardModel`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DashboardIdea {
    pub section: String,
    pub line_start: usize,
    pub line_end: usize,
    pub first_line: String,
}

/// Live data model powering the TUI Dashboard tab.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct DashboardModel {
    pub stage: String,
    pub main_file: String,
    pub engine: String,
    pub cited_bib_count: usize,
    pub total_bib_count: usize,
    pub unreferenced_labels_count: usize,
    pub undefined_refs_count: usize,
    pub health_audited: bool,
    pub ideas: Vec<DashboardIdea>,
    pub digest_publications: Vec<JournalPublication>,
    pub unparsed_sources_count: usize,
    pub open_todos_count: usize,
}

impl DashboardModel {
    /// Populate the dashboard model from current project root and app state.
    pub fn from_app(app: &App) -> Self {
        let stage = app
            .loaded_config
            .as_ref()
            .map(|c| c.project.stage.as_str().to_string())
            .unwrap_or_else(|| "draft".to_string());

        let main_file = app
            .loaded_config
            .as_ref()
            .map(|c| c.latex.main.to_string())
            .unwrap_or_else(|| "paper_draft.tex".to_string());

        let engine = if let Some(ref cfg) = app.loaded_config {
            format!("{} (configured)", cfg.latex.engine.as_str())
        } else if !app.global_settings.default_latex_engine.is_empty() {
            format!("{} (default)", app.global_settings.default_latex_engine)
        } else {
            "unset".to_string()
        };

        let mut health_audited = false;
        let mut cited_bib_count = 0;
        let mut total_bib_count = app.bib_file_entries.len();
        let mut unreferenced_labels_count = 0;
        let mut undefined_refs_count = 0;

        if let Some(ref root) = app.project_root {
            let draft_path = root.join(&main_file);
            let bib_path = root.join("references.bib");
            let bib_opt = if bib_path.is_file() {
                Some(bib_path.as_path())
            } else {
                None
            };
            if let Ok(report) = sil_latex::audit_manuscript(&draft_path, bib_opt) {
                health_audited = true;
                let (cited, total) = report.bib_citation_ratio();
                cited_bib_count = cited;
                total_bib_count = total;
                unreferenced_labels_count = report.unreferenced_labels_count;
                undefined_refs_count = report
                    .diagnostics
                    .iter()
                    .filter(|d| d.category == "undefined_reference")
                    .count();
            }
        }

        let tex_content = if !app.paper_draft_content.is_empty() {
            app.paper_draft_content.clone()
        } else if let Some(ref root) = app.project_root {
            let draft_path = root.join(&main_file);
            std::fs::read_to_string(draft_path).unwrap_or_default()
        } else {
            String::new()
        };

        let parsed_ideas = sil_latex::parse_idea_blocks(&tex_content);
        let ideas: Vec<DashboardIdea> = parsed_ideas
            .into_iter()
            .filter(|b| b.status != "resolved")
            .take(8)
            .map(|b| DashboardIdea {
                section: b.section_id.unwrap_or_else(|| "—".to_string()),
                line_start: b.line_start,
                line_end: b.line_end,
                first_line: b.content.lines().next().unwrap_or("").trim().to_string(),
            })
            .collect();

        let mut digest_publications = Vec::new();
        if let Some(ref root) = app.project_root {
            let db_path = sil_core::ProjectPaths::new(root).db();
            if let Ok(db) = sil_db::SilDb::open(&db_path) {
                if let Ok(pubs) = db.list_journal_publications() {
                    digest_publications = pubs;
                }
            }
        }

        let unparsed_sources_count = app.sources.iter().filter(|s| !s.parsed).count();
        let open_todos_count = ideas.len();

        Self {
            stage,
            main_file,
            engine,
            cited_bib_count,
            total_bib_count,
            unreferenced_labels_count,
            undefined_refs_count,
            health_audited,
            ideas,
            digest_publications,
            unparsed_sources_count,
            open_todos_count,
        }
    }
}

pub(crate) fn draw_dashboard(frame: &mut Frame, app: &mut App, area: Rect) {
    app.refresh_dashboard();
    let model = &app.dashboard;

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

    let (coverage_text, coverage_color) = if model.total_bib_count == 0 {
        (
            "0 references in references.bib".to_string(),
            Color::DarkGray,
        )
    } else if model.cited_bib_count == model.total_bib_count {
        (
            format!("{}/{} mentioned (100%)", model.cited_bib_count, model.total_bib_count),
            Color::Green,
        )
    } else {
        (
            format!(
                "{}/{} mentioned ({} unmentioned)",
                model.cited_bib_count,
                model.total_bib_count,
                model.total_bib_count - model.cited_bib_count
            ),
            Color::Yellow,
        )
    };

    let (label_text, label_color) = if !model.health_audited {
        ("Unchecked (no draft)".to_string(), Color::DarkGray)
    } else if model.unreferenced_labels_count == 0 && model.undefined_refs_count == 0 {
        ("OK (all labels matched)".to_string(), Color::Green)
    } else if model.unreferenced_labels_count > 0 && model.undefined_refs_count > 0 {
        (
            format!(
                "{} unreferenced, {} undefined",
                model.unreferenced_labels_count, model.undefined_refs_count
            ),
            Color::Red,
        )
    } else if model.undefined_refs_count > 0 {
        (
            format!("{} undefined reference(s)", model.undefined_refs_count),
            Color::Red,
        )
    } else {
        (
            format!("{} unreferenced label(s)", model.unreferenced_labels_count),
            Color::Yellow,
        )
    };

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
                &model.stage,
                Style::default().fg(Color::Green),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Main Draft: ", Style::default().fg(Color::Cyan)),
            Span::styled(&model.main_file, Style::default().fg(Color::Yellow)),
        ]),
        Line::from(vec![
            Span::styled("• Reference Coverage: ", Style::default().fg(Color::Cyan)),
            Span::styled(
                coverage_text,
                Style::default()
                    .fg(coverage_color)
                    .add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(vec![
            Span::styled("• Label References: ", Style::default().fg(Color::Cyan)),
            Span::styled(label_text, Style::default().fg(label_color)),
        ]),
        Line::from(vec![
            Span::styled("• Engine: ", Style::default().fg(Color::Cyan)),
            Span::styled(&model.engine, Style::default().fg(Color::Reset)),
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
    let mut idea_lines = vec![
        Line::from(vec![Span::styled(
            "# -- X -- # Idea & TODO Notes",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    if model.ideas.is_empty() {
        idea_lines.push(Line::from(vec![Span::styled(
            "No active # -- X -- # ideas or TODO blocks found.",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for (idx, idea) in model.ideas.iter().take(8).enumerate() {
            let label = format!(
                "{}. [{} / Lines {}-{}]: ",
                idx + 1,
                idea.section,
                idea.line_start,
                idea.line_end
            );
            idea_lines.push(Line::from(vec![
                Span::styled(label, Style::default().fg(Color::Yellow)),
                Span::styled(&idea.first_line, Style::default().fg(Color::Reset)),
            ]));
        }
    }

    idea_lines.push(Line::from(""));
    idea_lines.push(Line::from(vec![Span::styled(
        "Tip: Surround notes with # -- X -- # in paper_draft.tex for AI agents.",
        Style::default().fg(Color::Reset),
    )]));

    let idea_block = Block::default()
        .title(" [2] Active Ideas & TODO Blocks (# -- X -- #) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Yellow));
    frame.render_widget(Paragraph::new(idea_lines).block(idea_block), top_chunks[1]);

    // 3. Top Journal Digest Feed
    let mut digest_lines = vec![
        Line::from(vec![Span::styled(
            "Top Peer-Reviewed Journal Feed (Crossref / Nature / IEEE)",
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(""),
    ];

    if model.digest_publications.is_empty() {
        digest_lines.push(Line::from(vec![Span::styled(
            "No digest cached. Run Settings digest query or sil source digest.",
            Style::default().fg(Color::DarkGray),
        )]));
    } else {
        for pub_item in model.digest_publications.iter().take(6) {
            let tag = match pub_item.year {
                Some(y) if !pub_item.journal.is_empty() => format!("• [{} {}] ", pub_item.journal, y),
                Some(y) => format!("• [{}] ", y),
                None if !pub_item.journal.is_empty() => format!("• [{}] ", pub_item.journal),
                None => "• ".to_string(),
            };
            digest_lines.push(Line::from(vec![
                Span::styled(tag, Style::default().fg(Color::Green)),
                Span::styled(&pub_item.title, Style::default().fg(Color::Reset)),
            ]));
        }
    }

    digest_lines.push(Line::from(""));
    digest_lines.push(Line::from(vec![Span::styled(
        "Run Settings digest query or sil source digest to update feed.",
        Style::default().fg(Color::Reset),
    )]));

    let digest_block = Block::default()
        .title(" [3] Literature Digest (Top Journals) ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::Magenta));
    frame.render_widget(
        Paragraph::new(digest_lines).block(digest_block).wrap(Wrap { trim: true }),
        bottom_chunks[0],
    );

    // 4. Scientist Command Center & Shortcut Guide
    let guide_lines = vec![
        Line::from(vec![Span::styled(
            "Daily Scientist Helper Shortcuts (5 Navigation Tabs)",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )]),
        Line::from(vec![Span::styled(
            format!(
                "  Unparsed sources: {}  |  Open TODOs: {}",
                model.unparsed_sources_count, model.open_todos_count
            ),
            Style::default().fg(Color::DarkGray),
        )]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  1-5 / Tab / Shift+Tab",
                Style::default().fg(Color::Yellow),
            ),
            Span::styled(
                "  Switch: 1.Dash, 2.Sources, 3.Refs, 4.Draft, 5.Settings",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  ? / F1", Style::default().fg(Color::Yellow)),
            Span::styled(
                "               Open mode-aware keyboard help overlay anywhere",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  b / p / P", Style::default().fg(Color::Yellow)),
            Span::styled(
                "            Sources/Refs: Add source (b), Add ref (p), Promote (P)",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  v / e", Style::default().fg(Color::Yellow)),
            Span::styled(
                "                View refs/sort venue (v), Edit field/section (e)",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  m / X", Style::default().fg(Color::Yellow)),
            Span::styled(
                "                Sort by draft similarity (m), Recompute scores (X)",
                Style::default().fg(Color::Reset),
            ),
        ]),
        Line::from(vec![
            Span::styled("  y / i / s / t", Style::default().fg(Color::Yellow)),
            Span::styled(
                "        Sort references: Year (y), Index (i), Source (s), Title (t)",
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

#[cfg(test)]
mod tests {
    use super::*;
    use camino::Utf8PathBuf;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use tempfile::tempdir;

    fn render_to_string(app: &mut App) -> String {
        let backend = TestBackend::new(120, 40);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal.draw(|f| draw_dashboard(f, app, f.area())).unwrap();
        let buffer = terminal.backend().buffer();
        let mut out = String::new();
        for y in 0..buffer.area.height {
            for x in 0..buffer.area.width {
                let cell = &buffer[(x, y)];
                out.push_str(cell.symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn test_empty_or_missing_draft_ideas_no_panic() {
        let mut app = App::new(None);
        app.refresh_dashboard();
        assert!(app.dashboard.ideas.is_empty());
        let rendered = render_to_string(&mut app);
        assert!(rendered.contains("No active # -- X -- # ideas or TODO blocks found"));
    }

    #[test]
    fn test_draft_with_two_idea_blocks() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let draft_path = root.join("paper_draft.tex");
        let tex_content = r#"\section{Introduction}
# -- X -- #
[TODO: id=idea-1, status=open]
First idea content line
# -- X -- #

\section{Methods}
# -- X -- #
[TODO: id=idea-2, status=in_progress]
Second idea content line
# -- X -- #
"#;
        std::fs::write(&draft_path, tex_content).unwrap();

        let mut app = App::new(Some(root));
        assert_eq!(app.dashboard.ideas.len(), 2);
        assert_eq!(app.dashboard.ideas[0].section, "Introduction");
        assert_eq!(app.dashboard.ideas[0].first_line, "First idea content line");
        assert_eq!(app.dashboard.ideas[1].section, "Methods");
        assert_eq!(app.dashboard.ideas[1].first_line, "Second idea content line");

        let rendered = render_to_string(&mut app);
        assert!(rendered.contains("First idea content line"));
        assert!(rendered.contains("Second idea content line"));
        assert!(rendered.contains("Lines 2-5"));
        assert!(rendered.contains("Lines 8-11"));
    }

    #[test]
    fn test_audit_unmatched_labels_does_not_claim_ok() {
        let dir = tempdir().unwrap();
        let root = Utf8PathBuf::from_path_buf(dir.path().to_path_buf()).unwrap();
        let draft_path = root.join("paper_draft.tex");
        let tex_content = r#"\documentclass{article}
\begin{document}
\label{fig:unreferenced}
\end{document}
"#;
        std::fs::write(&draft_path, tex_content).unwrap();

        let mut app = App::new(Some(root));
        assert!(app.dashboard.unreferenced_labels_count > 0);

        let rendered = render_to_string(&mut app);
        assert!(!rendered.contains("OK (all labels matched)"));
        assert!(rendered.contains("unreferenced label"));
    }

    #[test]
    fn test_empty_digest_list_empty_state_copy() {
        let mut app = App::new(None);
        app.refresh_dashboard();
        assert!(app.dashboard.digest_publications.is_empty());

        let rendered = render_to_string(&mut app);
        assert!(rendered.contains("No digest cached"));
        assert!(rendered.contains("sil source digest"));
        assert!(!rendered.contains("Quantum Advantage"));
        assert!(!rendered.contains("self-attention baseline"));
        assert!(!rendered.contains("Stage 5 (Polish"));
        assert!(!rendered.contains("IEEE TPAMI"));
    }
}
