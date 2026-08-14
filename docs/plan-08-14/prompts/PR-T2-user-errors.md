# PR-T2 — UserError catalog + status mapping

Copy the block below into an agent session (worktree-isolated if parallel with D1).

---

## Role

You are the **error engineer** for scientist-in-loop. Ship ONLY PR-T2.

## Goal

Give scientists a short, actionable status line. Introduce `UserError { code, title, hint, retry }` in `sil-core` and map the most common TUI/CLI failures through it. Raw `Debug` / anyhow chains stay in logs or `--json`, not the status bar.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.8, KD-12
- Today: `App.status_message` is a free-form `String`. Job failures interpolate `{err}`. Doctor/CLI use `SilUi` + anyhow.
- `sil-core` already has `SilError`. Do not replace it; **map** to `UserError` at the surface.
- Do **not** put `CommandId` (Ratatui-adjacent) in `sil-core`. Store `retry` as `Option<&'static str>` (e.g. `"retry-last-job"`).

## Shared invariants

1. Minimal diff; no drive-by refactors.
2. Never auto-commit.
3. No new features, jobs, or tabs.
4. `--json` output may keep machine detail; human TUI/CLI title is the new contract.
5. Clippy clean on touched crates.

## Requirements

1. Add `sil_core::UserError` (code, title, hint, optional retry id). Display impl = `title`.
2. Add a small mapper for at least:
   - `crossref.rate_limited` / HTTP 429
   - `network.offline` / connection failed
   - `latex.engine_missing`
   - `parse.marker_missing`
   - `sqlite.busy`
   - `parse.failed`
   - `project.not_found`
   - `lock.held` (title/hint only; T6 will call it)
3. TUI: job failure + a few existing status assignments (`Estimate error`, fetch/parse fail) use `UserError.title` on the status bar. Keep the hint available (`App.last_user_error`) so `?` can show it later if cheap; otherwise document as residual.
4. Do not rewrite every `status_message = format!(...)` in one PR. Hit **job outcomes** and **one CLI doctor-facing path is enough if you also touch doctor** — actually leave doctor humanization to O3. T2 owns the type + TUI job mapping.
5. Unit tests:
   1. 429 / “rate limit” text maps to `crossref.rate_limited`.
   2. Missing project maps to `project.not_found`.
   3. `UserError` Display equals `title`, not the Debug of an inner anyhow.
   4. TUI test: a failed job sets `status_message` to the title, not a Rust type dump.

## Out of scope

- Doctor hint field / `--repair-db` (O3, T5)
- Persistent jobs (T3)
- Palette (D1) except optional retry string ids
- Rewriting MCP error JSON

## Verify

```bash
cargo test -p sil-core -p sil-tui
cargo clippy -p sil-core -p sil-tui --all-targets -- -D warnings
```

## Deliverable

`UserError` definition, mapper table, which TUI paths now use it, residual unmapped `status_message` sites.
