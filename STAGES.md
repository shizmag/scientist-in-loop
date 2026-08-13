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

## Stage 10 — MCP surface collapse (19 → 6) ✅
Collapsed MCP tool list from 19 fine-grained tools to 6 workflow-oriented tools (`sil_context`, `sil_sources`, `sil_cite`, `sil_draft`, `sil_review`, `sil_propose`). Behavior parity preserved via action/flags dispatch; hard cut of old names; docs honesty updated across README, STAGES, and skills.

## Stage 11 — Crash-Safe Durability & System Robustness (Wave 08-12) ✅
Plan: `docs/pr-plan-08-12/pr-plan.md`. ADR: `docs/adr/ADR-013-crash-safe-robustness.md`.
- **Atomic Writes**: `sil_core::write_atomic` / `write_atomic_str` standard across all durable file writes (bib, paper draft, config, structure, workspace lock, reviews, settings, cache).
- **SQLite WAL & Integrity**: Enforced `PRAGMA journal_mode = WAL; busy_timeout = 5000;` across all DB opens; added `sqlite integrity` doctor check.
- **Data Loss-Free Re-parse**: Transactional upsert `upsert_parsed_with_references` and `ParseOptions { allow_reparse }`; failed re-parses preserve existing index and FTS data.
- **API Retries & HTTPS**: Exponential backoff retry wrapper (3 attempts) across CrossRef, DOI, arXiv, and OpenReview; arXiv API endpoint migrated to `https://`.
- **Atomic PDF Downloads**: `.part` temporary file download + atomic `os.replace` + exponential retry on HTTP 429/5xx and network errors.
- **TUI Robustness & Async Estimate**: `catch_unwind` panic isolation on all background worker threads; converted manuscript L0 estimate into a non-blocking background job (`run_estimate_job`).

## Stage 12 — Three-surface use-case layer (`sil-app`) ✅
Plan: `docs/plan-sil-app/pr-plan.md`. ADR: `docs/adr/ADR-014-sil-app-usecase-layer.md`.
- **`sil-app` Crate**: Centralized sync use-case layer (`AppContext`, `AppError`, `upsert_bib`, `promote_bib`, `fetch_source`) to prevent feature/policy drift across CLI, MCP, and TUI surfaces without creating dependency cycles with the `sil` binary crate.
- **Unified Bib Writers**: Unified `upsert_bib` (always enforces `preserve_cite_key = true`, atomic write, draft markers `% [sil: tui-added]`, commit proposal) and `promote_bib` (strips draft markers, updates references, commit proposal).
- **Unified Source Fetch**: Centralized `fetch_source` orchestrating atomic PDF/source download, optional parsing via Marker, and richest official BibTeX resolution (`target` DOI/arXiv -> document DOI/arXiv/metadata resolver -> `upsert_bib(draft=false)`).
- **Surface Adapter Alignment**:
  - CLI: `sil source cite --append` / `--promote` and `sil source fetch` delegate to `sil-app`. `cite` output remains quiet (no commit proposal stdout printed).
  - MCP: `sil_cite` (upsert / promote) and `sil_sources` (fetch) delegate to `sil-app`. MCP fetch automatically upserts official BibTeX when resolved; parse errors surface on response (`parse_error`) without swallowing failures.
  - TUI: Explicit bibliography actions (append/promote) and background fetch job delegate to `sil-app` with `parse=false`.
- **Residual Drift**: Search still FTS-only on CLI vs dense RAG on MCP; rank embedder settings differ across surfaces; TUI hydration apply (`jobs.rs`) still updates `references.bib` directly.




