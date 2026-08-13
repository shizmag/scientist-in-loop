# PR-B2 — Background digest refresh

Copy the block below into an agent session. **Depends on A1 + B1.**

---

## Role

You are the **jobs engineer** for scientist-in-loop. Ship ONLY PR-B2.

## Goal

While the Dashboard tab is shown, if the digest cache is older than the configured interval (hours ≥ 1) and an effective query exists, refresh Crossref in a panic-isolated background job and store rows in `journal_digest`.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-13/pr-plan.md` §5.3, KD-5–KD-8
- Settings helpers from B1: `effective_digest_query`, `effective_digest_refresh_hours`
- Fetch: `sil_parse::fetch_journal_publications` / `sil_parse::journal_digest` (same as `crates/sil/src/commands/digest.rs`)
- Store: `SilDb::save_journal_publication`, `list_journal_publications`
- Schema already has `journal_digest.fetched_at` (`crates/sil-db/src/schema.rs`) but list/save may not expose it
- TUI jobs: `crates/sil-tui/src/app/jobs.rs` — `catch_unwind`, job history `J`, in-flight sets
- Dashboard model from A1 should display the cached list

## Shared invariants

1. Minimal diff. Reuse job chrome. Do not invent a daemon or CLI watcher.
2. Never auto-commit.
3. One in-flight digest job. Empty effective query → never spawn.
4. Refresh interval is in **hours**, minimum 1. No sub-hour polling.
5. Panic-isolate the worker (`catch_unwind`), same as hydrate/parse/estimate.
6. Prefer unit tests; clippy clean on touched crates.

## Requirements

1. Expose cache freshness from SQLite:
   - Either return `fetched_at` on `JournalPublication` / a small meta struct, or add `SilDb::digest_last_fetched_at() -> Option<String>`.
   - Prefer **no new table**. `MAX(fetched_at)` is enough.
2. Add `JobKind::Digest` (name may vary) + `queue_digest_refresh`.
3. When `ActiveTab::Dashboard` is shown (on tick or on tab enter), if:
   - effective query is `Some`,
   - no digest job in flight,
   - last fetch missing **or** age ≥ refresh hours,
   then queue the job.
4. Worker:
   - `fetch_journal_publications(query, 10)` (CLI default limit),
   - `save_journal_publication` for each item,
   - send success/fail through the existing outcome channel.
5. On success, reload dashboard digest rows. On failure, job history + status line (do not toast-spam).
6. Settings change: if the user saves a new query/interval, allow the next dashboard visit/tick to re-evaluate (do not require TUI restart). Do **not** refresh immediately on every keystroke.
7. Tests:
   1. Empty query → `queue` is a no-op.
   2. Fresh cache (fetched_at within interval) → no second spawn.
   3. In-flight set prevents duplicate jobs.
   4. `digest_last_fetched_at` / save+list round-trip (sil-db).
8. Do not change `sil source digest` behavior except that both paths keep writing the same table.

## Out of scope

- Digest row selection / Enter fetch (C3)
- Reader keys
- OS cron / launchd
- Multiple watch queries
- Refreshing on `sil status` or MCP

## Verify

```bash
cargo test -p sil-tui -p sil-db -p sil-parse
cargo clippy -p sil-tui -p sil-db --all-targets -- -D warnings
```

## Deliverable

Job kind, freshness API, when a refresh is scheduled, residual “needs network so e2e is mock/unit only”.
