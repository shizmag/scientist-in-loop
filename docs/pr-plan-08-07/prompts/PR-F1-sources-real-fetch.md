# PR-F1 — Sources real fetch on `a`

## Role

Ship ONLY PR-F1.

## Goal

Sources key `a` performs real download (DOI/arXiv/URL) via `sil_parse::fetch_source_target`, not register-stub only.

## Requirements

1. Replace register-only modal path with background fetch job.
2. On success: write under sources/, DB upsert, reload_sources; optional parse queue.
3. On failure: status + history outcome (even if F2 modal not ready).
4. Parity with MCP/CLI classify behavior.
5. Help overlay: `a` real fetch. Shift+A stub NOT required (KD-17).

## Out of scope

- Job history modal UI (F2); full keybinding redesign

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Files, status strings, residual risk.
