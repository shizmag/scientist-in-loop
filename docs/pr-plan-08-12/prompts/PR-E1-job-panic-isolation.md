# PR-E1 — TUI job panic isolation

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **tui-engineer** for scientist-in-loop. Ship ONLY PR-E1.

## Goal

If a TUI background worker panics, the job must complete as a failure and `in_flight_*` must clear. Today a panic means no channel send and a stuck hydrating/parsing/fetching/similarity state forever.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §E1, KD-15
- Jobs: `crates/sil-tui/src/app/jobs.rs` — five `std::thread::spawn` sites (hydrate ref, hydrate source, parse, fetch, similarity)
- Estimate is still synchronous — **leave it for E2**
- Precedent: `crates/sil-parse/src/checkers/mod.rs` already uses `catch_unwind` for bib checkers
- Poll paths already turn `Err` into a failed `JobOutcome` and clear in-flight sets

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. Do not change job UI chrome, history cap, or retry modal bindings.
4. Do not make estimate async (E2).
5. Prefer a small shared helper over five copy-pasted `catch_unwind` blocks.

## Requirements

1. Wrap **all five** current worker bodies in `catch_unwind(AssertUnwindSafe(...))`.
2. On panic: still `send` a failure result. Error string must start with the stable prefix `worker panicked:` (include payload if it is `&str` / `String`, else a generic message).
3. Poll paths must then: clear the corresponding `in_flight_*` entry, push a failed `JobOutcome`, and keep `retry_payload` when one already exists so `J` retry still works.
4. Preferred helper: `spawn_job` (or similar) that takes a closure returning `Result<T, String>` and maps panic → `Err("worker panicked: …")`.
5. Test: a `pub(crate)` / `#[cfg(test)]` hook that runs a panicking worker through the helper (or a focused App test) and asserts after poll that the matching `in_flight_*` is empty and a failed outcome exists.

## Out of scope

- Async estimate (E2)
- New job kinds
- Keymap / help redesign
- Atomic writes (A2)
- Changing hydration resolve logic

## Verify

```bash
cargo test -p sil-tui --lib
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Files changed, helper signature, how panic is tested, residual risk.
