# PR-G1 — Parent author F1 campaign

## Role

You are a quality agent for sil-parse / golden. Ship ONLY PR-G1.

## Goal

Lift BEE-RAG and HiChunk **parent authors F1** to ≥ **0.75** without golden gate regressions or negative-pattern pollution.

## Repo context

- Scorecard: `tests/golden_dataset/reports/candidate_scorecard.md` (per-fixture table; ignore H1 "Baseline" title)
- Current: BEE-RAG authors F1 **0.53**, HiChunk **0.46**
- EVAL: `tests/golden_dataset/EVALUATION.md` parent_authors_f1
- Gold: fixtures under `tests/golden_dataset/fixtures/` (or equivalent layout)
- Extraction: sil-parse parent metadata / xberg / byline paths

## Requirements

1. **Root-cause appendix first:** gold vs current authors for BEE-RAG + HiChunk (citation bleed, markdown artifacts, byline miss).
2. Minimal fix in sil-parse (or related) without tanking other fixtures' high F1.
3. Re-run score / export current_extraction if needed.
4. Unit tests for fixed byline cases.
5. If ≥0.75 unreachable without gate harm: document residual and stop (no scope creep).

## Out of scope

- TUI/MCP; ONNX; field precision (G2)

## Verify

```bash
# Prefer existing golden score script or:
cargo test -p sil-parse --lib
# + project golden score command if documented in tests/golden_dataset/README.md
cargo clippy -p sil-parse --all-targets -- -D warnings
```

## Deliverable

Root-cause notes, scorecard delta, residual risk.
