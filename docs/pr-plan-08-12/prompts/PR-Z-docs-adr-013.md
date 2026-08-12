# PR-Z — Docs, STAGES Stage 11, ADR-013

Copy the block below into an agent session. **Last after code PRs.**

---

## Role

Docs agent. Ship ONLY PR-Z.

## Goal

Docs claim only what 08-12 code does: crash-safe writes, WAL, bounded retry, panic-isolated TUI jobs, async estimate. No exclusive-lock or embed-cache-PK claims.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §Z, KD-18, residuals
- Update: `STAGES.md`, `README.md`, new `docs/adr/ADR-013-crash-safe-robustness.md`
- Cross-link `docs/pr-plan-08-12/`
- MCP tool count is **6** (Stage 10). Do not regress.

## Shared invariants

1. No product code / behavior changes.
2. Honest residuals (see below).
3. Never auto-commit.

## Requirements

1. `STAGES.md`: add **Stage 11** ✅ summarizing Wave 08-12 (atomic writes, WAL + busy_timeout, doctor integrity **report**, re-parse preserve, API retry + HTTPS, PDF `.part` replace, TUI panic isolation + async estimate). Keep Stage 9 leftover note honest if those tracks are still unfinished.
2. Write `docs/adr/ADR-013-crash-safe-robustness.md`:
   - Status: Accepted
   - Context: mid-write truncate, SQLITE_BUSY, delete-then-reparse, fail-fast 429, uncaught TUI panics
   - Decision: KD table from the plan (soft concurrency; advisory lock stays)
   - Residuals (must appear):
     - Advisory lock is still last-writer-wins
     - Embed-cache PK is still `content_hash` only
     - Doctor does **not** rebuild a corrupt DB
     - Windows `rename` replace is unproven (no Windows CI this wave)
3. README: short **Durability** note (atomic writes, WAL, retry/HTTPS, background estimate). Link the plan + ADR-013.
4. Do not claim exclusive locking, `is_busy` enforcement, GPU, or embed-cache PK fix.
5. MCP tool count remains 6 wherever counts are stated.

## Out of scope

- Logic changes
- Renumbering older ADRs
- Stage 9 leftover implementation

## Verify

```bash
rg -n 'exclusive lock|flock|embed-cache PK fixed|composite primary key' README.md STAGES.md docs/adr/ADR-013-crash-safe-robustness.md || true
# Expect: residuals may mention these as NOT done; no “we fixed embed-cache PK”
rg -n '6 workflow-oriented|6 tools' README.md STAGES.md
```

## Deliverable

Files changed, Stage 11 blurb, residual list in ADR-013.
