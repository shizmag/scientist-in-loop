# Autonomous agent prompts — 2026-08-09 Wave 09-08 / Stage 9

Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Do not combine PRs unless serial and explicitly chained |
| **Worktree isolation** | Prefer isolated git worktrees when parallel |
| **Shared preamble** | Every prompt is self-contained |
| **Commit policy** | Never auto-commit; Sci-Action proposals only |
| **Done criteria** | Green verify commands + residual risk note |
| **Do not expand scope** | Out of scope is hard forbidden |

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit; Sci-Action proposals only.
3. Estimate path is **read-only** on `paper_draft.tex`.
4. ARS is inspiration only — original sil skill prose + attribution (CC-BY-NC awareness).
5. `mode=onnx` only with session+tokenizer; never claim ONNX on fallback.
6. Prefer unit tests co-located; clippy clean.

## Parallel waves

```text
Wave 0:  A1 | B1 | B2 | E3 | R1
Wave 1:  A2 | C0 | D1 | E1 | R2 (after R1)
Wave 2:  C1 | C2 | D2 | R3 | E2
Wave 3:  C3 | R4 | E4? | G1
Wave 4:  F1 | F2 | G2 | D3?
Wave 5:  B3? | Z
```

## Prompt index

See filenames `PR-*.md` in this directory; full DAG in parent plan.
