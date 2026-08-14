# Autonomous agent prompts — 2026-08-14 Stage 14 / scientist-facing TUI

Copy-paste ready prompts for **one agent per PR**. Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Do not ask one agent to do multiple PRs unless serial and you explicitly chain them |
| **Worktree isolation** | Prefer isolated git worktrees when running parallel PRs in the same wave |
| **Shared preamble** | Every prompt is self-contained — agents must not depend on chat history |
| **Commit policy** | Agent may create a local commit **only if** the user asked; default = leave unstaged/staged summary |
| **Done criteria** | Green tests listed in the prompt + short summary: files, behavior, residual risk |
| **Do not expand scope** | Anything under “Out of scope” is forbidden even if “obvious” |

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors outside PR scope.
2. Never auto-commit. Sci-Action proposals only. Atomic write ≠ git commit.
3. **Same five TUI tabs.** New UI is palette, modals, banners, empty states. No sixth tab. No web/Tauri GUI. No `sil daily`. No new MCP tool.
4. Reuse `sil_app::upsert_bib` / `sil_app::fetch_source` / `sil_latex::update_or_insert_idea_block` / existing TUI job chrome / `run_estimate_job`. Do not fork those policies.
5. Prefer unit tests co-located; clippy `-D warnings` on touched crates.
6. Once **D1** exists, new verbs register a `CommandId` and run through `App::dispatch`.
7. Once **T2** exists, user-visible errors go through `UserError` (title on the status bar; raw `Debug` stays in logs / `--json`).
8. Once **T1** exists, covered TUI mutations (delete source, delete bib, note insert, cite insert) snapshot the undo journal.
9. Digest/add-source composite is **fetch + parse** (`parse=true`). Do **not** auto-open the reader.
10. No OS daemon / cron / launchd. Jobs persist as `.sil/jobs.json` only (T3). Lock is PID-live + confirm, not `flock`.

## Parallel waves

```text
Wave 0 (parallel):  D1 | T2 | O3
Wave 1 (parallel):  D2 | O1 | T4 | T6     (after Wave 0 deps)
Wave 2 (parallel):  R1 | R2 | R3 | T1
Wave 3 (parallel):  R4 | W1 | W2 | T3 | T5
Wave 4 (parallel):  D3 | D4 | W3 | W4 | O2
Wave 5:             V then Z
```

Spine if time-boxed: **D1 + T2 + R1 + R2 + T1 + O1 + T6**. Slip-ok: **D4, W3, W4, O2**.

## Prompt index

| PR | File | Depends on | Parallel with |
|----|------|------------|---------------|
| **D1** Command palette | [PR-D1-command-palette.md](PR-D1-command-palette.md) | — | T2, O3 |
| **T2** UserError catalog | [PR-T2-user-errors.md](PR-T2-user-errors.md) | — | D1, O3 |
| **O3** Doctor-as-guide | [PR-O3-doctor-guide.md](PR-O3-doctor-guide.md) | T2 | D1 |
| **D2** Empty states | [PR-D2-empty-states.md](PR-D2-empty-states.md) | D1, T2 | O1, T4, T6 |
| **O1** First-run wizard | [PR-O1-first-run-wizard.md](PR-O1-first-run-wizard.md) | D1, O3 | D2, T4, T6 |
| **T4** Conflict banner | [PR-T4-conflict-banner.md](PR-T4-conflict-banner.md) | T2 | D2, O1, T6 |
| **T6** Honest lock | [PR-T6-honest-lock.md](PR-T6-honest-lock.md) | T2 | D2, O1, T4 |
| **R1** Fetch+parse | [PR-R1-fetch-parse.md](PR-R1-fetch-parse.md) | D1 | R2, R3, T1 |
| **R2** Note section picker | [PR-R2-note-section.md](PR-R2-note-section.md) | — | R1, R3, T1 |
| **R3** Derived badges | [PR-R3-source-badges.md](PR-R3-source-badges.md) | R1 | R2, T1 |
| **T1** Undo journal | [PR-T1-undo.md](PR-T1-undo.md) | — | R1, R2, R3 |
| **R4** Cite into section | [PR-R4-cite-section.md](PR-R4-cite-section.md) | R2, T1 | W1, W2, T3, T5 |
| **W1** Estimate report | [PR-W1-estimate-view.md](PR-W1-estimate-view.md) | D1 | R4, W2, T3, T5 |
| **W2** Build + error jump | [PR-W2-build-errors.md](PR-W2-build-errors.md) | D1 | R4, W1, T3, T5 |
| **T3** Persistent jobs | [PR-T3-job-persist.md](PR-T3-job-persist.md) | T2 | R4, W1, W2, T5 |
| **T5** Doctor `--repair-db` | [PR-T5-repair-db.md](PR-T5-repair-db.md) | O3 | R4, W1, W2, T3 |
| **D3** Keymap aliases | [PR-D3-keymap-aliases.md](PR-D3-keymap-aliases.md) | D1 | D4, W3, W4, O2 |
| **D4** Mouse dispatch | [PR-D4-mouse.md](PR-D4-mouse.md) | D1 | D3, W3, W4, O2 |
| **W3** Grounding modal | [PR-W3-grounding.md](PR-W3-grounding.md) | R4 | D3, D4, W4, O2 |
| **W4** Proposal diff | [PR-W4-proposal-diff.md](PR-W4-proposal-diff.md) | W2, T1 | D3, D4, W3, O2 |
| **O2** Demo project | [PR-O2-demo-project.md](PR-O2-demo-project.md) | O1 | D3, D4, W3, W4 |
| **V** Verification | [PR-V-verify.md](PR-V-verify.md) | all code PRs | — |
| **Z** Docs / ADR-016 | [PR-Z-docs-adr-016.md](PR-Z-docs-adr-016.md) | V | last |

## Subagent roles

| Role | PR |
|------|-----|
| Palette engineer | D1, D3 |
| Empty-state engineer | D2 |
| Mouse engineer | D4 |
| Ingest engineer | R1 |
| Reader-note engineer | R2 |
| Badge engineer | R3 |
| Cite-insert engineer | R4 |
| Undo engineer | T1 |
| Error engineer | T2 |
| Jobs engineer | T3 |
| Watch engineer | T4 |
| Lock engineer | T6 |
| Doctor engineer | O3, T5 |
| Onboarding engineer | O1, O2 |
| Estimate engineer | W1 |
| Build engineer | W2 |
| Grounding engineer | W3 |
| Diff engineer | W4 |
| Verifier | V |
| Docs agent | Z |

## Product defaults (KD)

- Five tabs only. Palette is the spine (`:` / `Ctrl-K`).
- Digest Enter / add-source = `sil_app::fetch_source(parse=true)`. No auto-open reader.
- Note `n` picks a draft section (`IdeaBlock.section_id`); parser already supports it.
- Cite-into-section inserts `\cite{key}` via a new `sil-latex` helper. Sci-Action = `EditDraft`.
- Badges are derived (parsed / in bib / cited). No new SQLite columns.
- Undo: `.sil/undo/`, last 10, TUI mutations only. Never `git checkout`.
- `UserError` in `sil-core`. Status bar shows `title`.
- Jobs persist in `.sil/jobs.json`. Running-at-quit → `stale`. Retry from start.
- Conflict banner on mtime; lock is PID-live + confirm, not flock.
- `doctor --repair-db` backups then rebuilds. Never delete `sources/`.
- Wizard when `project_root` is `None`. `sil init --demo` is synthetic.
- Estimate / build / ground / diff are thin TUI views over existing engines. Never auto-commit.
- MCP stays **6** tools.
