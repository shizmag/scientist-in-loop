# Autonomous agent prompts — 2026-08-12 Stage 11 / robustness

Copy-paste ready prompts for **one agent per PR**. Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Do not ask one agent to do multiple PRs unless serial and you explicitly chain them |
| **Worktree isolation** | Prefer isolated git worktrees when running in parallel |
| **Shared preamble** | Every prompt is self-contained — agents must not depend on chat history |
| **Commit policy** | Agent may create a local commit **only if** the user asked; default = leave unstaged/staged summary |
| **Done criteria** | Green tests listed in the prompt + short summary: files, behavior, residual risk |
| **Do not expand scope** | Anything under “Out of scope” is forbidden even if “obvious” |

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors outside PR scope.
2. Never auto-commit; Sci-Action proposals only. Atomic write ≠ git commit.
3. Soft concurrency only: atomic files + SQLite WAL. Do **not** honor `is_busy` or add exclusive `flock`.
4. Do **not** change embed-cache PRIMARY KEY (explicit residual; ADR-013).
5. Prefer unit tests co-located; clippy clean on touched crates.
6. Default `sil source parse` of an already-parsed file still fails (C1 only changes the **force** path).
7. Estimate path stays **read-only** on `paper_draft.tex`.

## Parallel waves

```text
Wave 0 (parallel):  PR-A1 | PR-B1 | PR-D1 | PR-D2 | PR-E1
Wave 1:             PR-A2 (after A1) | PR-C1 | PR-E2 (after E1)
Wave 2:             PR-F1 (after A2, B1, C1, D1, D2; E2 soft)
Wave 3:             PR-Z (last)
```

## Prompt index

| PR | File | Depends on | Parallel with |
|----|------|------------|---------------|
| **PR-A1** Atomic write primitive | [PR-A1-atomic-write.md](PR-A1-atomic-write.md) | — | B1, D1, D2, E1 |
| **PR-A2** Adopt atomic writes | [PR-A2-adopt-atomic-writes.md](PR-A2-adopt-atomic-writes.md) | A1 | C1, E2 |
| **PR-B1** SQLite WAL + integrity | [PR-B1-sqlite-wal.md](PR-B1-sqlite-wal.md) | — | A1, D1, D2, E1 |
| **PR-C1** Re-parse without data loss | [PR-C1-reparse-preserve.md](PR-C1-reparse-preserve.md) | — | A2, E2 |
| **PR-D1** API retry + HTTPS | [PR-D1-api-retry-https.md](PR-D1-api-retry-https.md) | — | A1, B1, D2, E1 |
| **PR-D2** PDF download atomic | [PR-D2-download-atomic.md](PR-D2-download-atomic.md) | — | A1, B1, D1, E1 |
| **PR-E1** TUI job panic isolation | [PR-E1-job-panic-isolation.md](PR-E1-job-panic-isolation.md) | — | A1, B1, D1, D2 |
| **PR-E2** Async TUI estimate | [PR-E2-async-estimate.md](PR-E2-async-estimate.md) | E1 | A2, C1 |
| **PR-F1** E2E crash / recovery | [PR-F1-e2e-recovery.md](PR-F1-e2e-recovery.md) | A2, B1, C1, D1, D2 | — |
| **PR-Z** Docs / ADR-013 | [PR-Z-docs-adr-013.md](PR-Z-docs-adr-013.md) | all must-ship | last |

## Product defaults (KD)

- `write_atomic` / `write_atomic_str` in sil-core; same-dir temp + fsync + rename
- WAL + `busy_timeout=5000` + `foreign_keys=ON` + `synchronous=NORMAL` on every `SilDb::open`
- Doctor `sqlite integrity` is **read-only** (no rebuild)
- Force re-parse: `ParseOptions { allow_reparse: true }`; never `remove_source` first
- Retry: 3 attempts, 250/500/1000 ms, ±20% jitter, cap 2 s; 429/5xx/transport only
- arXiv export URL is HTTPS
- PDF: `{dest}.part` then `os.replace`; unlink `.part` on failure
- TUI workers: `catch_unwind`; estimate is a background job
- Workspace lock stays advisory (last writer can still win)
