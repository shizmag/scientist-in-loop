# PR-A1 — Live dashboard

Copy the block below into an agent session (worktree-isolated if parallel with B1).

---

## Role

You are the **dashboard engineer** for scientist-in-loop. Ship ONLY PR-A1.

## Goal

Replace the hardcoded Dashboard tab with a live `DashboardModel` so a scientist opening `sil tui` sees real manuscript health, real `# -- X -- #` ideas, and the cached journal digest. Pane 4 stays the keymap.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-13/pr-plan.md` §5.1, KD-1, KD-3, KD-4
- Mock today: `crates/sil-tui/src/ui/dashboard.rs` (dummy Stage 5, dummy TODOs, dummy Nature/IEEE titles)
- Live bits already used: `sil_latex::audit_manuscript` + `bib_citation_ratio()`
- Ideas: `sil_latex::parse_idea_blocks` on `paper_draft.tex` (same as `sil paper todo`)
- Digest cache: `SilDb::list_journal_publications` (`crates/sil-db/src/lib.rs`)
- Stage / engine: `Config` (`config.project.stage`, `config.latex.engine`, `config.latex.main`)
- Health types: `sil_core::ManuscriptHealthReport` / `HealthDiagnostic`

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Never auto-commit.
3. Keep the 2×2 layout and pane titles. Do not add a tab or a `sil daily` command.
4. Pane 4 is still a shortcut guide. A single factual count line is allowed (unparsed / open TODOs / digest age). No “you should write the intro” coaching.
5. Prefer unit tests co-located; clippy clean on touched crates.

## Requirements

1. Introduce a testable `DashboardModel` (name may vary) built from project root / loaded `App` state. **Do not** keep dummy strings in `draw_dashboard`.
2. **Health pane**
   - Stage from `config.project.stage` (`draft` / `prep` / `review` / `final`) — never the sil implementation “Stage 5”.
   - Main file + engine from config (fallback: `paper_draft.tex`, configured engine or “unset”).
   - Reference coverage from `audit_manuscript` (already wired).
   - Label status from the same report (`unreferenced_labels_count` / diagnostics). Green only if the audit says so. Do **not** hardcode “OK (all labels matched)”.
3. **Ideas pane**
   - Parse `paper_draft.tex` via `parse_idea_blocks`.
   - List open / in_progress blocks (skip `resolved` if status is present). Cap at a small N (≈8) so the pane does not overflow.
   - Each row: section (or “—”), line range, first content line.
   - Empty: keep the tip about surrounding notes with `# -- X -- #`.
4. **Digest pane**
   - Render `list_journal_publications` if a DB is open.
   - Empty: “No digest cached. Run Settings digest query or `sil source digest`.”
   - Do **not** call Crossref in this PR (B2). Age/stale is optional until B2 exposes `fetched_at`.
5. **Shortcuts pane**
   - Keep existing key lines.
   - Optional top line of counts only, e.g. unparsed source count + open TODO count.
6. Load hooks: when the TUI opens a project (or `R` reload), refresh the model so the dashboard is not one-shot stale vs Sources/Draft tabs.
7. Unit tests (no extra crates):
   1. Empty / missing draft → empty ideas, no panic.
   2. Draft with two `# -- X -- #` blocks → two idea rows with line ranges.
   3. Audit with unmatched labels → health does not claim OK.
   4. Empty digest list → empty-state copy, not dummy Nature titles.
8. Update `crates/sil-tui/src/ui/tests.rs` if it snapshots dummy dashboard copy.

## Out of scope

- Digest HTTP, settings fields, background job (B1/B2)
- Reader `b` / `n` (C1/C2)
- Digest row selection / Enter (C3)
- New CLI commands, MCP tools, JSON dumps
- Writing sessions, close ritual, experiment watchers

## Verify

```bash
cargo test -p sil-tui -p sil-latex
cargo clippy -p sil-tui --all-targets -- -D warnings
```

Confirm `dashboard.rs` contains no “Quantum Advantage”, “self-attention baseline”, or “Stage 5 (Polish”.

## Deliverable

Files changed, `DashboardModel` fields, how health/ideas/digest are loaded, residual “digest age unknown until B2” note.
