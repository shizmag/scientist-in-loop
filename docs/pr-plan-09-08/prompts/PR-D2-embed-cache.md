# PR-D2 — Embed cache

Parent design: [../pr-plan.md](../pr-plan.md).

## Role

Ship ONLY PR-D2. See parent plan §8 for full requirements.

## Goal

Embed cache as specified in `docs/pr-plan-09-08/pr-plan.md`.

## Requirements

1. Follow Key Decisions KD-1..KD-14 and never auto-commit.
2. Match existing Rust style; minimal diff; no drive-by refactors.
3. Unit tests co-located; clippy clean on touched crates.
4. Complete acceptance criteria in parent plan for PR-D2.

## Out of scope

Anything not listed for PR-D2 in the parent plan.

## Verify

```bash
cargo test --workspace
cargo clippy --workspace --all-targets -- -D warnings
```

(Use crate-scoped tests when parent plan lists them.)

## Deliverable

Files changed, behavior summary, residual risk.
