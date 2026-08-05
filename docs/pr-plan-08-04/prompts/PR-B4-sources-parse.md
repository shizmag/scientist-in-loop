# PR-B4 — Sources parse action (stretch / optional)

Copy the block below into an agent session after B2+B3. Skip if orchestrator marks stretch deferred.

---

## Role

Focused implementer. Ship ONLY PR-B4 after B2+B3. Skip entirely if orchestrator marks stretch deferred.

## Goal

Allow parsing/extracting the selected source from the TUI without dropping to CLI.

## Repo context

- Parse orchestration: `crates/sil-parse/src/batch.rs` (`parse_one` / `parse_many`)
- UI shows `[✓ Parsed]` / `[Unparsed]` badges from DB state
- Marker/xberg can be slow — must not freeze UI (background job + B2 chrome)

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. TUI must not block on long parse work (background job).
3. Prefer unit tests co-located with modules; keep clippy clean on touched crates.

## Requirements

1. Sources-list key (suggest `e` for extract/parse — avoid colliding with Refs `P` promote):
   - Queue parse for selected source in background
   - On success: reload source row + refs; status OK
   - On failure: status with error
2. Disable or no-op with message if already parsed unless force modifier (optional Shift+e)
3. Reuse job chrome from B2 (`parsing: N` or generic background jobs)
4. Minimal tests with stubbed parse if project patterns allow

## Out of scope

- Batch parse all unparsed
- Changing Marker/xberg algorithms

## Verify

```bash
cargo test -p sil-tui --lib
cargo test -p sil-parse --lib
```

## Deliverable

Key chosen, job lifecycle, known limitations (Marker install required, etc.).
