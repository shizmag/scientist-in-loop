# Implementation stages

All stages below are complete. `cargo test --workspace` and
`cargo clippy --workspace --all-targets -- -D warnings` are green.

## Stage 0 — Workspace skeleton ✅
Multi-crate layout, domain types in `sil-core`, `sil --help` command surface,
stub libraries with real APIs, unit tests.

## Stage 1 — `sil init` ✅
Templates, exact project layout, git init, SQLite create, first commit proposal
(never auto-commit). E2E init coverage.

## Stage 2 — Config / structure / status ✅
Typed `config.yaml` & `structure.yaml`, validation, `sil status`.

## Stage 3 — Parse / search / Marker ✅
SQLite+FTS5, `sil parse` (path + noninteractive multi-select), `sil search`,
Marker Python integration (stubbable), progress abstraction.

## Stage 4 — Git proposals / log ✅
Commit proposals with Sci-Action trailers, `sil log`.

## Stage 5 — Build / fetch / context / polish ✅
`sil build`, `sil source fetch`, `sil context` + dynamic skills, colored UX,
root README, final e2e suite.

## Stage 6 — MCP Server & Local ONNX RAG Integration ✅
`sil mcp` stdio JSON-RPC server (`crates/sil-mcp`), parent-child section/paragraph chunking, feature-gated ONNX embeddings & cross-encoder reranking (hash fallback by default), BM25+Dense RRF & HyDE hybrid search, structured LaTeX TODO governance. (Tool count evolved; see Stage 9 for current surface.)

## Stage 7 — Interactive TUI Refactoring: Sources & Unified Settings ✅
Interactive 4-tab Ratatui TUI command center. Dedicated Sources tab (#3) with paginated pretty Markdown reading, auto-fetch link modal, parse status indicators, word/reference statistics, reference viewer, renaming, and deletion confirmation. Unified vertical Settings window (#4) combining Global, RAG, Caches, and Local settings with visual section dividers.

## Stage 8 — Wave D: ONNX RAG Truth, Agent Bib Write, TUI Finish, Quality & Release Hygiene ✅
Feature-gated real ONNX inference for embedder and reranker (`--features onnx`) with default hash fallback and honest doctor diagnostics (`sil project doctor`). Comprehensive agent bib write path via MCP tools (`sil_upsert_bib`, `sil_promote_bib`, `sil_parse_source`, structure depth, `sil_rank_draft`). Finished TUI async job chrome with Sources fetch on `a`, job history log and retry modal on `J`, and non-blocking draft-reference similarity computation on `X`/`m`. CI golden gate and formatting enforcement (`cargo fmt --check`). Residual hard-fixture author F1 cliffs tracked in Stage 9.

## Stage 9 — Trust, Co-Author, Estimate, Ship (Wave 09-08) 🚧
Plan: `docs/pr-plan-09-08/`. Delivered in-tree so far:

- **Docs honesty**: MCP tool count and ONNX feature/fallback wording aligned with code.
- **Sci-Action**: `estimate-paper`, `ground-claims`; advisory `.sil/workspace.lock`.
- **Estimate (ARS-inspired, sil-native)**: `agent/skills/review.md` + rubrics/personas; L0 `sil paper estimate`; MCP `sil_estimate_paper` (read-only on draft; writes `.sil/reviews/` only).
- **Co-author MCP**: `sil_edit_section`, `sil_ground_claims` (never auto-commit).
- **19 MCP tools** total.

Remaining plan tracks (quality fixtures B*, embed cache D2, Releases F*, TUI estimate R4, ADR-012 closer Z): see `docs/pr-plan-09-08/pr-plan.md`.


