# ADR-015: Honest Daily Command Center & Reader Verbs

## Status
Accepted (Wave 08-13 / Stage 13)

## Context
Prior to Wave 08-13, `sil tui` claimed to serve as a daily command center, but its Dashboard tab (tab 1) relied on static mock values: hardcoded project stage ("Stage 5"), static label health status ("OK"), dummy `# -- X -- #` TODO blocks, and hardcoded sample journal titles. Scientists opening `sil tui` could not trust panes 1–3 for actual daily project status.

Additionally:
- Literature digest (`sil source digest`) operated as an isolated CLI one-shot query. Rows were saved to `journal_digest`, but the TUI never rendered or refreshed them, nor did global or local settings support a digest query or refresh interval.
- The TUI Markdown source reader (tab 2) was scroll-only. Capturing a citation or parking a claim from a paper required exiting the reader or relying on AI agent tools.
- CLI proposals had suggested a separate `sil daily` ritual command or a `sil daily --json` agent orientation dump, which would duplicate existing TUI surfaces and MCP tools (`sil_context`).

## Decisions (KD-1 through KD-15)

1. **Dashboard is the Daily View (KD-1)**: The TUI Dashboard tab is the single daily command center. No secondary CLI ritual command (`sil daily`) or command alias was added.
2. **No JSON Twin (KD-2)**: Agents continue using `sil_context` and the 6 workflow-oriented MCP tools. No duplicate JSON orientation dump or extra MCP orientation tool was created.
3. **Live Dashboard Panes (KD-3)**: Retained the four-pane layout (Health, Ideas, Digest, Shortcuts) while eliminating all dummy strings:
   - Pane 1 (Health): Displays live stage from `config.yaml`, LaTeX engine and main file, bib coverage ratio, and unmatched label count from `audit_manuscript`.
   - Pane 2 (Ideas): Parses and renders real active `# -- X -- #` TODO and idea blocks from `paper_draft.tex`.
   - Pane 3 (Digest): Renders live publication rows from `journal_digest` with age and refresh status.
   - Pane 4 (Shortcuts): Preserves command shortcuts keymap.
4. **Shortcuts Pane Scope (KD-4)**: Retained Pane 4 strictly as a keyboard shortcut guide and factual counts summary, avoiding intrusive coaching or recommendation logic.
5. **Settings-Backed Digest Query (KD-5)**: Digest query resolution follows precedence: `LocalSettings.digest_query` if non-empty, otherwise `GlobalSettings.digest_query`. If both are empty, auto-refresh is disabled.
6. **Refresh Interval (KD-6)**: Added `digest_refresh_hours: u32` in `GlobalSettings` (`~/.config/sil/settings.yaml`), defaulting to **1** hour (minimum 1 hour; values < 1 are clamped).
7. **TUI-Lifetime Background Refresh (KD-7)**: Auto-refresh executes only while the Dashboard tab is active and the cache age exceeds `digest_refresh_hours`. Spawns a non-blocking background worker (`JobKind::Digest`) using existing job history chrome (`J`).
8. **Manual CLI Trigger Preserved (KD-8)**: `sil source digest [query]` remains available for manual CLI execution and populates the same `journal_digest` table.
9. **Reader Citation Verb `b` (KD-9)**: Added key `b` to the Markdown reader view to append the current source to `references.bib` via `sil_app::upsert_bib` (`draft: true`), enforcing cite-key preservation and `% [sil: tui-added]` tagging without auto-committing.
10. **Reader Note Capture Verb `n` (KD-10)**: Added key `n` to the Markdown reader view, opening `ModalCaptureNote`. Committing a non-empty note inserts a `% # -- X -- #` block into `paper_draft.tex` via `sil_latex::update_or_insert_idea_block`, tagged `from-source` with content starting with `from: <filename>` and `author_type: human`.
11. **Derived Literature States (KD-11)**: Avoided adding "triaged" or "reading" columns to SQLite; "in bib" and "cited in draft" remain dynamically derived facts.
12. **Existing Sci-Action Trailers (KD-12)**: Reused existing action categories (`UpdateBibliography` for `b`, `EditDraft` for `n`) rather than creating new proposal variants.
13. **Digest Row Ingest via `Enter` (KD-13)**: Pressing `Enter` on a publication row in the Dashboard digest pane queues an async source fetch via `sil_app::fetch_source` (`parse=false`). Status updates inform the user to parse and read on tab 2; the reader is not automatically opened.
14. **Unified UI & Modal Patterns (KD-14)**: Reused existing ratatui modal patterns (`ModalCaptureNote`), job execution chrome, and shortcut help overlays (`HelpMode::ReadingSourceMd`).
15. **Never Auto-Commit (KD-15)**: All write paths (`references.bib`, `paper_draft.tex`, settings YAML) perform atomic file operations via `sil_core::write_atomic_str` and generate commit proposals for human review.

## Residuals

- **TUI-Lifetime Refresh Only**: Digest auto-refresh runs exclusively while `sil tui` is open with the Dashboard tab active. No OS daemon, launchd job, or CLI status-triggered refresh was introduced.
- **Single Effective Digest Query**: The digest system evaluates one effective query string rather than supporting a multi-query watch list.
- **TUI Fetch `parse=false`**: Fetching a source from the Dashboard digest queue downloads the document with `parse=false`; document text extraction and reading remain on tab 2.
- **No Source Triage States / Section Pins**: No formal source triage state machine ("triaged / read / cited") or `structure.yaml` section-pinning was added.
- **No Experiment / `data/` Integration**: External experiment logs, training runs, and `data/` directories remain decoupled from the daily dashboard view.
- **Search & Rank Surface Drift**: Search on CLI remains FTS-only while MCP supports RAG hybrid search (carried over from Stage 12).
