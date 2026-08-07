# PR-D1 — Real ONNX embed + rerank (feature-gated)

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust implementer for scientist-in-loop. Ship ONLY PR-D1.

## Goal

Feature-gate real ONNX inference for `OnnxEmbedder` / `OnnxReranker` behind cargo feature `onnx`. Default builds stay on hash/token fallback. Never report `mode=onnx` without session + tokenizer.

## Repo context

- Workspace: scientist-in-loop
- Primary: `crates/sil-db/src/onnx.rs`, `crates/sil-db/Cargo.toml`, workspace `Cargo.toml`, `crates/sil/Cargo.toml` features re-export only
- Settings paths: `sil_core::RagSettings` resolve_embedder_path / resolve_reranker_path
- Parent plan: `docs/pr-plan-08-07/pr-plan.md` §D1

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Never claim ONNX when fallback active.
3. Default CI must not require models or ort load.
4. Prefer unit tests co-located; clippy clean on touched crates.

## Requirements

1. Workspace pin `ort` + `tokenizers` (prefer ort with download-binaries/vendored so CI needs no system ORT).
2. `sil-db` feature: `onnx = ["dep:ort", "dep:tokenizers"]` (exact feature flags as crate requires).
3. `sil` re-export only: `onnx = ["sil-db/onnx"]`. Do **not** add features on sil-mcp/sil-tui for v1.
4. On-disk layout: dir with `model.onnx` + `tokenizer.json`, or `.onnx` file with sibling tokenizer.
5. Embed pipeline: tokenize → session → pool (mask mean if rank-3) → L2. `mode=onnx` only if session AND tokenizer loaded.
6. Rerank pipeline: cross-encoder pair encode → scalar score.
7. CPU-only EP; apply `num_threads` when API allows.
8. `RagBackend` / `backend()` API: `Onnx { dim }` | `Fallback { reason }` (FeatureDisabled, ModelPathMissing, MissingTokenizer, SessionLoadFailed, …).
9. **Forbidden:** raw mean-pool over model bytes reported as onnx.
10. Unit tests: fallback always works without models/feature.
11. Optional tiny fixtures under `crates/sil-db/tests/fixtures/` with `#[cfg(feature="onnx")]` and/or `#[ignore]`.
12. **Mandatory dual-runtime spike:** `cargo build -p sil --features onnx` must succeed with current xberg `ner-onnx`, or document incompatibility and **do not merge** without user-approved constraint (KD-20).
13. Document feature in module docs; HF export recipe note for full models under `~/.cache/sil/models/` (no bootstrap script required).

## Out of scope

- Auto-download models; doctor/TUI chrome (D2); MCP API; CUDA; raw mean-pool-as-onnx

## Verify

```bash
cargo test -p sil-db
cargo clippy -p sil-db --all-targets -- -D warnings
cargo build -p sil --features onnx   # must succeed or document KD-20 block
# optional:
cargo test -p sil-db --features onnx
```

## Deliverable

Files changed, behavior before/after, ort versions chosen, dual-runtime spike result, residual risk.
