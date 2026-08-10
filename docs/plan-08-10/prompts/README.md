# Autonomous agent prompts — 2026-08-10 Wave 08-10 / MCP collapse

Parent design: [../pr-plan.md](../pr-plan.md).

## Dispatch rules

| Rule | Detail |
|------|--------|
| **One agent per PR** | Ship M1 alone unless M2 docs split is explicitly requested |
| **Worktree isolation** | Prefer isolated git worktree when parallel with other work |
| **Shared preamble** | Every prompt is self-contained |
| **Commit policy** | Never auto-commit product changes without user request; Sci-Action proposals only inside sil |
| **Done criteria** | Green verify commands + residual risk note |
| **Do not expand scope** | Out of scope is hard forbidden |

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit from MCP tools; Sci-Action proposals only.
3. Estimate path is **read-only** on `paper_draft.tex`.
4. Hard cut: **6 tools only** — no old-name aliases.
5. ONNX honesty: dense path only with feature + models; never claim ONNX on fallback.
6. Prefer unit tests co-located; clippy clean.

## Wave

```text
Wave 0:  M1 (collapse + docs honesty)
Optional: M2 (extra docs/ADR sweep) only if M1 is split
```

## Prompt index

| PR | File | Depends |
|----|------|---------|
| **M1** MCP collapse 19 → 6 | [PR-M1-mcp-collapse.md](PR-M1-mcp-collapse.md) | — |
