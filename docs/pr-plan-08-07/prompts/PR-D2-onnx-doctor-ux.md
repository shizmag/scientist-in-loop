# PR-D2 — Doctor / TUI honesty + model bootstrap hints

## Role

Ship ONLY PR-D2. Depends on D1.

## Goal

Honest Dense RAG status in `sil doctor` and TUI; never claim ONNX when fallback. Document HF export recipe only (script stretch).

## Requirements

1. Extend doctor `Check` with `extra: Option<serde_json::Value>` (additive).
2. Report dense_rag: mode onnx|fallback + reason + dim; intentional fallback `ok=true`.
3. TUI Settings RAG section + footer badge when fallback during similarity/search.
4. Help text: no unconditional "ONNX embeddings" on `X`.
5. Remove `/Volumes/happy-disk` from any docs you touch; use `~/.cache/sil/models`.
6. Stretch only: `--fix-rag` or bootstrap script.

## Out of scope

- Changing default CI to require models; implementing remote embed

## Verify

```bash
cargo test -p sil
cargo test -p sil-tui
cargo clippy -p sil -p sil-tui --all-targets -- -D warnings
# manual: sil doctor --json | jq '.checks[] | select(.name=="dense_rag")'
```

## Deliverable

Files, JSON sample, residual risk.
