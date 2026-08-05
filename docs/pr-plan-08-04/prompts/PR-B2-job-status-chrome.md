# PR-B2 — Background job status chrome

Copy the block below into an agent session. Prefer base branch with PR-A4.

---

## Role

Focused implementer. Ship ONLY PR-B2. Prefer base branch with PR-A4 (write serialization). If A4 not merged, do not reintroduce concurrent disk writes.

## Goal

Replace last-message-wins hydration status with visible in-flight counts and aggregate outcomes.

## Repo context

- `crates/sil-tui/src/app.rs`: `in_flight_hydration_keys`, `poll_background_hydration`, status_message
- Event loop ~100ms: `crates/sil-tui/src/lib.rs`
- Batch add (mark many + `p`/`a`) spawns many workers; user only sees last completion

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. TUI bib add stays non-blocking (local first + background hydrate).
3. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. Track:
   - `in_flight` count/set (already partially exists)
   - counters for completed ok / failed since last batch or rolling
   - optional `VecDeque` of last N outcomes (e.g. 20) with cite/title + ok/fail reason
2. Footer/status display:
   - While in flight: `hydrating: N` (and dirty flag as today)
   - When a batch drains to zero: one summary `✓ Metadata: 3 ok, 1 failed` (or similar)
3. Dedup skip: optional brief status `already hydrating …` if easy; else skip silently OK
4. Do not block UI thread; workers still only send results over channel
5. Unit tests for poll aggregating multiple results in one tick if feasible

## Out of scope

- Non-blocking similarity recompute (`X`)
- Full job retry UI (unless tiny)
- Sources fetch wiring (PR-B3)

## Verify

```bash
cargo test -p sil-tui --lib
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

UX strings documented; how batch add looks in footer.
