# PR-E1 — MCP bibliography write path

## Role

Ship ONLY PR-E1.

## Goal

Agent can upsert/promote `references.bib` via MCP without TUI; never git commit.

## Requirements

1. `SciAction::UpdateBibliography` (`update-bibliography`) and `PromoteBibliography` (`promote-bibliography`) in sil-core; FromStr/as_str/tests.
2. Tools `sil_upsert_bib` (string BibTeX only; draft default false; preserve_cite_key default true) and `sil_promote_bib`.
3. Path: `ProjectPaths::new(root).join(rel::REFERENCES)` — no invented `references()` helper.
4. Reuse `upsert_bib_entry_with_options`; re-read disk before write; pretty + completeness.
5. Return JSON: wrote, cite_key, replaced, proposal, never_committed: true.
6. Tests: temp project + git init; HEAD unchanged after tools.

## Out of scope

- Auto-hydrate network from MCP; structured field objects; auto-commit

## Verify

```bash
cargo test -p sil-core
cargo test -p sil-mcp
cargo clippy -p sil-core -p sil-mcp --all-targets -- -D warnings
```

## Deliverable

Files, tool schemas, residual risk.
