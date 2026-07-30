# ADR-004: Native Model Context Protocol (MCP) Server Integration with Local ONNX Models

## Context

`scientist-in-loop` (`sil`) provides a structured workspace for scientific manuscripts, literature search via SQLite/FTS5, LaTeX section tracking, and commit proposal governance. To allow external LLM agents (Antigravity, Claude Desktop, Cursor, etc.) to interact directly with `sil` projects via standard protocol, we need a native Model Context Protocol (MCP) server interface.

To adhere to `sil`'s core philosophy of being 100% local, privacy-first, and reproducible offline, dense embedding generation and cross-encoder re-ranking run using **local ONNX runtime models**.

## Decision

1. **Dedicated `sil-mcp` Crate**: We introduce `crates/sil-mcp` to handle JSON-RPC 2.0 transport over `stdio`. The binary `sil` exposes this via `sil mcp`.
2. **Layered Core Integration**: `sil-mcp` delegates directly to existing crates (`sil-db`, `sil-core`, `sil-agent`, `sil-latex`, `sil-git`) without duplicating domain logic.
3. **100% Local ONNX Inference Engine**: Dense embeddings (`bge-small-en-v1.5` ONNX) and cross-encoder re-ranking (`ms-marco-MiniLM-L-6-v2` ONNX) are executed locally via ONNX Runtime (`ort` crate). Model weights are cached locally in `~/.cache/sil/models/` or custom user directories.
4. **Configurable Local ONNX Paths**: Custom ONNX directory paths (`onnx_models_dir`), explicit model file paths (`onnx_embedder_path`, `onnx_reranker_path`), execution provider (`cpu`, `coreml`, `cuda`), thread counts, and model cache paths are configurable in `GlobalSettings` (`~/.config/sil/settings.yaml`), local `Config` (`.sil/config.yaml`), and `sil settings` TUI.
5. **Literature RAG with Parent-Child Chunking**: `sil-db` is upgraded with `source_chunks` and `chunks_fts` tables to support parent section expansion, BM25 + Local ONNX Dense RRF hybrid ranking, and HyDE search modes.
6. **Structured Async TODO Governance**: `# -- X -- #` LaTeX blocks in `paper_draft.tex` are indexed with status, priority, section tags, and author provenance.
7. **Strict Governance & Commit Proposals**: MCP tools modify workspace state safely. The `sil_propose_commit` tool formats `Sci-Action:` proposals for human approval and **never** auto-commits.
8. **5-Stage Phased Multi-Subagent Execution**: Implementation is structured in sequential stages, tested and committed independently at each stage boundary.

## Status

Accepted / Proposed for implementation.

## Consequences

- AI agents can natively query literature, inspect workspace context, run skills, and propose commits.
- Zero network/API dependencies for literature vector search; 100% offline local processing.
- Flexible ONNX model placement on disk (custom folders, shared paths, or explicit files).
