# PR-H2 — CI golden + fmt

## Role

Ship ONLY PR-H2. Soft after G* for thresholds; fmt independent.

## Goal

PR CI runs `cargo fmt --all -- --check` and a **PR-blocking golden** job (gate FAIL / negative pollution fails the PR). Keep latency reasonable (cache, skip heavy marker).

## Requirements

1. `.github/workflows/ci.yml`: fmt --check on PR.
2. Golden validate + score on PR (fail on pollution or gate FAIL).
3. Document local golden in `tests/golden_dataset/README.md`.
4. Scorecard title hygiene note (Candidate vs Baseline).

## Out of scope

- Windows CI; crates.io publish

## Verify

```bash
cargo fmt --all -- --check
# local golden commands from README
```

## Deliverable

CI yaml diff, residual risk.
