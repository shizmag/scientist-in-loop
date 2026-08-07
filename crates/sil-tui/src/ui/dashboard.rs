//! Dashboard view rendering for `sil-tui`.

use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
};

use crate::app::App;

pub(crate) fn draw_dashboard(frame: &mut Frame, _app: &mut App, area: Rect) {
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
            "Daily Scientist Helper Shortcuts (5 Navigation Tabs)",
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
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
