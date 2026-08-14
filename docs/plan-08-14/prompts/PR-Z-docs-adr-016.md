# PR-Z — Docs, STAGES Stage 14, ADR-016

Copy the block below into an agent session. **Last, after PR-V.**

---

## Role

Docs agent. Ship ONLY PR-Z.

## Goal

Docs claim only what 08-14 code does: scientist-facing TUI (palette, empty states, fetch+parse, section note/cite, badges), visible robustness (undo, UserError, jobs persist, conflict/lock banners, `--repair-db`), onboarding (wizard, doctor hints, optional demo), writing handshake (estimate view, build errors, thin ground/diff if shipped). No GUI. No daemon. No hard flock. No auto-commit. MCP stays 6.

## Repo context

- Parent plan: `docs/plan-08-14/pr-plan.md` §12, KD table, residuals §11
- Update: `STAGES.md`, `README.md`, new `docs/adr/ADR-016-scientist-facing-tui.md`
- Cross-link `docs/plan-08-14/`
- MCP tool count is **6** (Stage 10). Do not regress.
- Stage 13 / ADR-015 stay accurate. Note 08-14 **reversals**: fetch+parse, section picker, derived badges, `--repair-db`, visible lock.

## Shared invariants

1. No product code / behavior changes.
2. Honest residuals (copy from the plan). Honest about slipped PRs (D4/W3/W4/O2) — only document what code does.
3. Never auto-commit.

## Requirements

1. `STAGES.md`: add **Stage 14** summarizing Wave 08-14. Mark slip-ok items only if they shipped (check the tree). Keep Stage 9 leftover note honest if those tracks are still unfinished.
2. Write `docs/adr/ADR-016-scientist-facing-tui.md`:
   - Status: Accepted
   - Context: cockpit without search, reading dead-ends, silent races, empty first run, CLI-only writing tools
   - Decision: KD-1–KD-27 from the plan (command registry spine; five tabs; fetch+parse no auto-open; derived badges; undo journal; UserError; jobs.json; honest lock; repair-db; wizard)
   - Explicit reversals vs ADR-013 / ADR-015
   - Residuals (must appear):
     - Split-pane source+draft not added
     - No GitHub Releases / prebuilts
     - Lock is not `flock` / not NFS-safe
     - Single digest query; TUI-lifetime refresh
     - No experiment/`data/` dashboard
     - Search/rank surface drift leftover from Stage 12
     - Embed-cache PK still `content_hash`
     - Windows atomic rename unproven
     - W4 is uncommitted diff only (if shipped)
     - Auto-open reader remains off
3. `README.md`:
   - TUI: palette `:` / `Ctrl-K`
   - Reader: `n` section picker; cite-into-section if shipped; `b` still bib upsert
   - Sources badges
   - Doctor: hints; `--repair-db`
   - `sil init --demo` only if O2 shipped
   - First-run wizard when no project
   - MCP tool count remains 6
   - Do **not** claim a GUI, daemon, hard lock, or auto-commit
4. Do not renumber older ADRs.

## Out of scope

- Logic changes
- Stage 9 leftover implementation
- Inventing features that slipped

## Verify

```bash
rg -n 'sil daily' README.md STAGES.md docs/adr/ADR-016-scientist-facing-tui.md || true
# Expect: only historical “we did not add sil daily” — not a command table row

rg -n '6 workflow-oriented|6 tools' README.md STAGES.md

rg -n 'daemon|launchd|Tauri|flock|auto-commit' README.md STAGES.md docs/adr/ADR-016-scientist-facing-tui.md || true
# Mentions must be negations / residuals, not product claims
```

## Deliverable

Files changed, Stage 14 blurb, ADR-016 path, README bullets that match **shipped** code only.
