# PR-E2 — Async TUI estimate

Copy the block below into an agent session. **Depends on E1** (`catch_unwind` / `spawn_job` helper).

---

## Role

You are a focused Rust **tui-engineer** for scientist-in-loop. Ship ONLY PR-E2.

## Goal

L0 manuscript estimate must not block the TUI event loop. It becomes a background job with the same shape as similarity: channel, in-flight flag, poll, panic isolation.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §E2, KD-16
- Today: `App::run_estimate_job` in `crates/sil-tui/src/app/jobs.rs` (~718–770) calls `sil_agent::run_heuristic_estimate` **on the event-loop thread**
- Pattern to copy: `enqueue_similarity_job` + `poll_background_similarity` + `in_flight_similarity` + `similarity_tx/rx`
- Types: `crates/sil-tui/src/app/types.rs` (`JobKind::Estimate` already exists)
- E1 should already provide a panic-isolating spawn helper — use it
- Estimate is **read-only** on `paper_draft.tex` (writes only under `.sil/reviews/` if the existing path already does)

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. Do not write `paper_draft.tex`.
4. Do not add L1 LLM calls or new estimate modes.
5. Do not invent new keybindings; keep whatever key already triggers estimate.

## Requirements

1. Add `estimate_tx/rx`, `in_flight_estimate: bool`, `poll_background_estimate`.
2. `run_estimate_job` only enqueues (sets in-flight, status “estimating…”). L0 runs on a worker via the E1 helper.
3. Second trigger while in flight → status `already estimating` (or equivalent); do not spawn another worker.
4. On success/failure: clear in-flight, push `JobOutcome { kind: JobKind::Estimate, ... }` (already used today).
5. Panic on the worker becomes a failed outcome (`worker panicked:`) and clears in-flight.
6. If help text implies estimate is instant/blocking, change it to “background job”.
7. Test: calling `run_estimate_job` returns without waiting for L0 (in-flight true before poll); after injecting a result (or running L0 on a tiny fixture + poll), in-flight is false.

## Out of scope

- L1 agent panel / ARS personas
- New MCP tools
- Changing L0 scoring
- Job history chrome redesign
- Exclusive locking

## Verify

```bash
cargo test -p sil-tui --lib
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Files changed, channel/field names, confirmation draft is not written, residual risk.
