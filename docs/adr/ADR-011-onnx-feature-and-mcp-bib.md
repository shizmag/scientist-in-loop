# ADR-011: Feature-Gated ONNX Inference, MCP Bib Write Governance, and Async TUI Jobs

## Status
Accepted

## Context
1. In Stage 6, `sil-db` claimed dense RAG capabilities, but embedding and reranking implementations fell back to FNV-hash token pooling without real ONNX runtime inference loading.
2. MCP tools enabled agents to query sources and inspect workspace context, but agents lacked authorization and tools to mutate `references.bib`, parse documents, or set section completion in `structure.yaml`.
3. TUI operations (such as source link fetching and reference-draft similarity recomputation) ran either as un-logged background tasks or synchronously blocked the UI thread.

## Decision Drivers
- **Honesty in Capabilities**: The CLI, doctor diagnostics, and TUI must never claim active ONNX inference when running on hash fallbacks.
- **Agent Safety**: AI agents operating via MCP must be capable of inserting and promoting bibliography entries while strictly observing the project's **no-auto-commit** invariant.
- **Responsive UI**: Long-running network or compute tasks (ingestion, similarity recomputation) must run asynchronously with visible status indicators and a dedicated job history/retry surface.

## Decision

### 1. Feature-Gated ONNX Inference (`--features onnx`)
- `sil-db` re-exports optional ONNX embedding and cross-encoder reranking under the `onnx` Cargo feature flag.
- When `onnx` is disabled or model files are missing from `~/.cache/sil/models/`, `OnnxEmbedder` and `OnnxReranker` explicitly report `RagBackend::Fallback { reason }`.
- `sil project doctor` outputs a structured `dense_rag` check reflecting active runtime mode, dimension, and model cache paths.

### 2. MCP Bibliography & Pipeline Write Governance
- Added `sil_upsert_bib` and `sil_promote_bib` MCP tools backed by `sil_core::bib::upsert_bib_entry_with_options`.
- MCP write actions generate structured proposal payloads containing `Sci-Action:` trailers (e.g. `Sci-Action: BibUpsert`, `Sci-Action: BibPromote`), leaving git commits to explicit user approval.
- Added `sil_parse_source` and `sil_set_structure` tools to complete agent autonomy over paper ingestion and section tracking.

### 3. Asynchronous TUI Job Chrome & History Log
- Extended `sil-tui` with an unified background job model (`JobKind::Hydrate`, `JobKind::Fetch`, `JobKind::Parse`, `JobKind::Similarity`).
- The Sources tab `a` shortcut enqueues asynchronous downloads via `sil_parse::fetch_source_target`.
- Similarity recomputation (`X` / `m`) runs asynchronously in a worker thread without blocking the TUI event loop.
- Pressing `J` opens a dedicated Job History modal with job execution outcomes and `r` / `Enter` retry keybindings for failed jobs.

## Consequences
- **Positive**: Zero hidden runtime dependencies in default builds; transparent health reporting in `sil project doctor`; autonomous agent bibliography workflow without auto-commit risks; responsive TUI experience.
- **Negative**: Full dense vector search requires explicit model downloads into `~/.cache/sil/models/` and building with `--features onnx`.
