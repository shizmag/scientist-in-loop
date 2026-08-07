# PR-G2 — Anchor field precision

## Role

Quality agent. Ship ONLY PR-G2. Parallel with G1.

## Goal

Lift `structure_predict_hallucination` (and similar weak fixtures) **field precision** from **65%** to ≥ **0.80** without negative-pattern pollution or gate FAIL.

## Requirements

1. Root-cause appendix: gold vs current fields (venue/year/title) for weak fixture(s).
2. Improve reference field extractors in sil-parse without polluting negatives.
3. Golden still 0 polluted refs; all macro gates PASS.
4. Unit tests for extracted field samples.

## Out of scope

- Parent author F1 (G1); TUI; ONNX

## Verify

```bash
cargo test -p sil-parse --lib
# + golden score as documented
cargo clippy -p sil-parse --all-targets -- -D warnings
```

## Deliverable

Root-cause, scorecard delta, residual risk.
