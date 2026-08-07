# PR-F2 — Job history modal + retry

## Role

Ship ONLY PR-F2. After F1 preferred.

## Goal

Generalize existing hydrate history into unified job history; key `J` modal + retry.

## Requirements

1. Promote `HydrationHistoryEntry` / `recent_hydration_outcomes` — **do not** invent a second ring.
2. Cap ≥20; kinds: hydrate | fetch | parse | similarity.
3. Key `J` modal: kind, label, status, timestamp, error snippet; Retry on Failed.
4. Grep key collisions; help documents `J`.
5. Optional `duration_ms` on outcomes.
6. Existing hydrate race/history tests green.

## Out of scope

- Async similarity implementation (F3); H1 module split

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Files, keybinding notes, residual risk.
