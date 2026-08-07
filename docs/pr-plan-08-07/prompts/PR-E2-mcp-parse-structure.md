# PR-E2 — MCP parse + structure depth

## Role

Ship ONLY PR-E2. Depends on E1 preferred (shared MCP patterns).

## Goal

Agents can parse sources and set full section completion without TUI.

## Requirements

1. `sil_parse_source` (path or source id) via existing sil-parse pipeline.
2. Deepen `sil_get_structure`: four-state `completion` enum; optional main_claim / secondary_points / required_content.
3. Keep `completed` bool backward-compat only; remove or no-op `word_count`.
4. Optional small `sil_rank_draft`.
5. Errors as CallToolResult::error.

## Out of scope

- Word targets schema expansion; TUI UI

## Verify

```bash
cargo test -p sil-mcp
cargo clippy -p sil-mcp --all-targets -- -D warnings
```

## Deliverable

Files, schema changes, residual risk.
