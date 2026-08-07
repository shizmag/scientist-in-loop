# Autonomous agent prompts — 2026-08-07 Wave D

Copy-paste ready prompts for **one agent per PR**. Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Do not ask one agent to do multiple PRs unless serial and you explicitly chain them |
| **Worktree isolation** | Prefer isolated git worktrees when running in parallel |
| **Shared preamble** | Every prompt is self-contained — agents must not depend on chat history |
| **Commit policy** | Agent may create a local commit **only if** the user asked; default = leave unstaged/staged summary |
| **Done criteria** | Green tests listed in the prompt + short summary: files, behavior, residual risk |
| **Do not expand scope** | Anything under "Out of scope" is forbidden even if "obvious" |

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors outside PR scope.
2. Never auto-commit; Sci-Action proposals only.
3. Bib writes: pretty + completeness-aware + cite-key preserve on hydrate; re-read disk before write.
4. Do not claim ONNX when fallback is active (`mode=onnx` only with session+tokenizer).
5. Prefer unit tests co-located; clippy clean on touched crates.
6. TUI: keep hydration non-blocking; extend same job pattern for fetch/similarity.
7. Similarity **recompute** is key **`X` only**; `m`/`c` remain sort-only.

## Parallel waves

```text
Wave 0 (parallel):  PR-D1 | PR-G1 | PR-G2
Wave 1 (after D1 for D2):  PR-D2 | PR-E1 | PR-F1
Wave 2a:  PR-E2 | PR-F2
Wave 2b (after F2 + D2):  PR-F3
Wave 3 (after F2+F3):  PR-H1
Wave 4:  PR-H2 then PR-H3
```

## Prompt index

| PR | File | Depends on | Parallel with |
|----|------|------------|---------------|
| **PR-D1** Real ONNX | [PR-D1-real-onnx.md](PR-D1-real-onnx.md) | — | G1, G2 |
| **PR-D2** Doctor/TUI honesty | [PR-D2-onnx-doctor-ux.md](PR-D2-onnx-doctor-ux.md) | D1 | E1, F1 |
| **PR-E1** MCP bib write | [PR-E1-mcp-bib-write.md](PR-E1-mcp-bib-write.md) | — | D2, F1 |
| **PR-E2** MCP parse/structure | [PR-E2-mcp-parse-structure.md](PR-E2-mcp-parse-structure.md) | E1 preferred | F2 |
| **PR-F1** Sources real fetch | [PR-F1-sources-real-fetch.md](PR-F1-sources-real-fetch.md) | — | E1, D2 |
| **PR-F2** Job history J | [PR-F2-job-history-retry.md](PR-F2-job-history-retry.md) | F1 preferred | E2 |
| **PR-F3** Async similarity | [PR-F3-async-similarity.md](PR-F3-async-similarity.md) | F2 + D2 | — |
| **PR-G1** Parent author F1 | [PR-G1-parent-author-f1.md](PR-G1-parent-author-f1.md) | — | D1, G2 |
| **PR-G2** Anchor field precision | [PR-G2-anchor-field-precision.md](PR-G2-anchor-field-precision.md) | — | D1, G1 |
| **PR-H1** TUI module split | [PR-H1-tui-module-split.md](PR-H1-tui-module-split.md) | F2+F3 | — |
| **PR-H2** CI golden + fmt | [PR-H2-ci-golden-fmt.md](PR-H2-ci-golden-fmt.md) | Soft after G* | early fmt OK |
| **PR-H3** Docs/STAGES/ADR | [PR-H3-docs-stage-adr.md](PR-H3-docs-stage-adr.md) | Last | — |

## Product defaults (KD)

- Feature `onnx` on sil-db; re-export only on `sil`
- MCP draft default false; never git commit
- ADR-007 keep parent-metadata; split-view → 008; Wave D → ADR-011
- Golden **PR-blocking** at H2; fmt every PR
- D1 blocked until ort links with xberg ner-onnx (or documented constraint approved)
- HF export recipe only for models (bootstrap script stretch)
