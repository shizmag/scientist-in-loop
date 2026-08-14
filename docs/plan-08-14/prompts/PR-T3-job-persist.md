# PR-T3 — Persistent job queue

Copy the block below into an agent session. **After T2.**

---

## Role

You are the **jobs engineer** for scientist-in-loop. Ship ONLY PR-T3.

## Goal

Persist TUI jobs to `.sil/jobs.json` (atomic). On TUI start, mark leftover `running` rows `stale` and allow Retry from `J`. No OS daemon. No resume of a half-written PDF (retry = start over).

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.9, KD-13
- Today: in-memory `recent_job_outcomes` ring (`JOB_HISTORY_CAP`). `J` modal already retries failed jobs from memory.
- Atomic write helper: `sil_core::write_atomic_str`.

## Shared invariants

1. Minimal diff. TUI-lifetime execution still; only the **record** persists.
2. Never auto-commit.
3. Cap 50 records.
4. Clippy clean.

## Requirements

1. Schema (JSON array), fields: `id`, `kind` (`fetch|parse|digest|estimate|build|hydrate|similarity` as already used), `label`, `status` (`running|ok|fail|stale`), `started`, `ended`, `error_code`.
2. On spawn: append/update `running`. On complete: `ok`/`fail` + `UserError.code` if any.
3. On TUI start: load file; any `running` → `stale`; populate `recent_job_outcomes` (or replace it from disk).
4. `J` Retry on `fail`/`stale` re-dispatches the original kind (existing retry payloads).
5. Gitignore `.sil/jobs.json` if it does not belong in git (yes — add to managed gitignore).
6. Unit tests:
   1. Restart with a `running` row marks it `stale`.
   2. Successful job writes `ok`.
   3. Cap 50.
   4. Corrupt JSON → empty queue + status warning, no panic.

## Out of scope

- Daemon / cron
- Partial PDF resume
- Changing job worker implementations

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

JSON schema, load/save points, gitignore, retry behavior for stale jobs.
