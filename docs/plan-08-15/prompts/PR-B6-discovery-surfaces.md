# PR-B6 - Discovery CLI, TUI, and MCP surfaces

## Role

Discovery surface engineer. Keep all surfaces thin over B5.

## Goal

Expose remote discovery and candidate triage consistently through CLI, TUI, and the existing `sil_sources` MCP tool without automatic bibliography mutation.

## Requirements

1. Add CLI commands for discover, runs/results, candidate list/show, shortlist/dismiss, fetch/parse, and explicit add-to-bib. Support human and complete JSON output.
2. Replace/globalize the TUI digest assumptions into a candidate inbox using persisted query-scoped runs. Show canonical venue with raw fallback and resolved/ambiguous/unknown evidence.
3. TUI states distinguish provider partial failure, candidate disposition, and acquisition. No hidden fetch/parse or auto-open unless the explicit command says so.
4. Add `sil_sources action=discover|candidates` to the existing tool with typed D2 schemas/results. Keep six tool names.
5. All three surfaces call B5 and the existing Stage-12 `sil-app` fetch/upsert use cases and produce equivalent identifiers/status/rank explanations.
6. Discovery never writes `references.bib`; explicit candidate add-to-bib uses the existing upsert path and proposal/never-commit contract.
7. Default output is compact; rank components/provider evidence are available on detail/JSON.

## Tests

Offline end-to-end fixture: query -> partial provider run -> ranked inbox -> shortlist -> fetch/parse -> explicit bib add. CLI JSON/text, TUI mode/state/badges/filter, MCP schemas/results, canonical/raw venue fallback, ambiguity, no bib mutation before explicit action, six-tool count.

## Out of scope

New providers/ranking, auto-citation insertion, universal venue prestige, live-network CI.

## Verify

```bash
cargo test -p sil -p sil-tui -p sil-mcp -p sil-app
cargo clippy -p sil -p sil-tui -p sil-mcp -p sil-app --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Workflow mapping/screens/JSON examples, parity results, no commit.
