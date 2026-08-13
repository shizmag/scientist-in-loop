# PR-Z — Docs, STAGES Stage 13, ADR-015

Copy the block below into an agent session. **Last, after PR-V.**

---

## Role

Docs agent. Ship ONLY PR-Z.

## Goal

Docs claim only what 08-13 code does: live dashboard, settings-backed digest refresh (TUI-lifetime), reader `b`/`n`, digest Enter → existing fetch. No `sil daily` command. No daemon. No writing-session / experiment / multi-project claims.

## Repo context

- Parent plan: `docs/plan-08-13/pr-plan.md` §12, KD table, residuals §11
- Update: `STAGES.md`, `README.md`, new `docs/adr/ADR-015-daily-command-center.md`
- Cross-link `docs/plan-08-13/`
- MCP tool count is **6** (Stage 10). Do not regress.
- Stage 12 / ADR-014 stay accurate.

## Shared invariants

1. No product code / behavior changes.
2. Honest residuals (copy from the plan).
3. Never auto-commit.

## Requirements

1. `STAGES.md`: add **Stage 13** ✅ summarizing Wave 08-13:
   - Live Dashboard (structure/audit/TODOs/digest cache)
   - Digest query + `digest_refresh_hours` in Settings; background refresh while Dashboard is shown
   - Reader `b` cite via `sil-app`; reader `n` `# -- X -- #` with `from:`
   - Digest Enter queues existing fetch (`parse=false`)
   - Explicitly: **no** `sil daily` command
2. Write `docs/adr/ADR-015-daily-command-center.md`:
   - Status: Accepted
   - Context: mock dashboard, one-shot digest, reader was scroll-only
   - Decision: KD-1–KD-15 from the plan (dashboard = daily view; no JSON twin; TUI-lifetime digest; verbs A+B)
   - Residuals (must appear):
     - No CLI/status-triggered digest refresh
     - Single query, not a watch list
     - TUI fetch still `parse=false`
     - No source triage states / no section pin
     - No experiment/`data/` integration
     - Search/rank surface drift leftover from Stage 12
3. `README.md`:
   - Dashboard bullet: panes are **live** (health, ideas, digest, keymap).
   - Settings: document digest query + refresh hours (global + local override).
   - Reader: `b` append source to bib; `n` park note.
   - **Remove or reword** `sil dashboard` / `sil daily` so it does not look like a CLI command. Point at `sil tui dashboard`.
   - MCP tool count remains 6 wherever counts are stated.
4. Do not claim cron/daemon digest, writing sessions, close-the-day, Sci-Action notebooks, or multi-project morning.

## Out of scope

- Logic changes
- Renumbering older ADRs
- Stage 9 leftover implementation

## Verify

```bash
rg -n 'sil daily' README.md STAGES.md docs/adr/ADR-015-daily-command-center.md || true
# Expect: only “we did not add sil daily” / reworded TUI mention — not a command table row

rg -n '6 workflow-oriented|6 tools' README.md STAGES.md

rg -n 'daemon|launchd|writing session|close the day' README.md STAGES.md docs/adr/ADR-015-daily-command-center.md || true
# Expect: residuals / non-goals, not “we shipped”
```

## Deliverable

Files changed, Stage 13 blurb, residual list in ADR-015.
