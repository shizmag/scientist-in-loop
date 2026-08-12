# ADR-013: Crash-Safe Durability and System Robustness

## Status
Accepted (Wave 08-12 / Stage 11)

## Context
Prior to Stage 11, scientist-in-loop relied on direct `std::fs::write` calls across TUI, MCP, CLI, and core crates, creating mid-write truncation risks during unexpected process exits or crashes. Additionally, SQLite connections used default journal modes without busy timeouts (leading to `SQLITE_BUSY` contention), force re-parsing in TUI removed source rows prior to text extraction (causing data loss on parse failure), API network calls lacked exponential backoff retry wrappers, PDF downloads mutated files directly without atomic temporary replacements, and TUI background threads lacked panic isolation.

## Decision

1. **Atomic Write Primitive**: Standardized all durable file writes (`references.bib`, `paper_draft.tex`, `.sil/config.yaml`, `.sil/structure.yaml`, `.sil/workspace.lock`, `.sil/reviews/*`, global settings, cache) on `sil_core::write_atomic` / `sil_core::write_atomic_str`. Writes write to a PID/nanosecond temporary file in the same directory, flush to disk via `sync_all()`, and atomically replace the destination via `fs::rename()`.
2. **SQLite WAL & Busy Timeout**: Enforced `PRAGMA journal_mode = WAL;`, `PRAGMA busy_timeout = 5000;`, `PRAGMA foreign_keys = ON;`, and `PRAGMA synchronous = NORMAL;` across all `SilDb::open` calls. Added `SilDb::integrity_check()` and exposed it via `sil doctor`.
3. **Transactional Re-parse without Data Loss**: Introduced `ParseOptions { allow_reparse: bool }` in `sil-parse` and added `SilDb::upsert_parsed_with_references` to execute `upsert_parsed` and `save_source_references` inside a single SQLite transaction. Removed pre-parse `remove_source` calls so failed re-parses preserve existing index and FTS data.
4. **API Retries & arXiv HTTPS**: Wrapped CrossRef, DOI, arXiv, and OpenReview HTTP requests in `with_retry` (3 attempts, 250ms base backoff with 2x multiplier capped at 2000ms, failing fast on 4xx / parse errors). Migrated arXiv endpoint to `https://export.arxiv.org`.
5. **Atomic PDF Download & Retry**: `download_pdf.py` writes HTTP response bodies into a `.part` temporary file before calling `os.replace` for atomic replacement, with exponential backoff on HTTP 429/5xx and network errors.
6. **TUI Panic Isolation & Async Estimate**: Enclosed TUI background workers in `std::panic::catch_unwind(AssertUnwindSafe(...))` to map panics to failed job outcomes instead of crashing the UI loop. Converted L0 manuscript estimate (`run_estimate_job`) into a non-blocking background thread worker with channel status polling.

## Residuals

- **Advisory Workspace Lock**: `.sil/workspace.lock` remains advisory (last writer wins at the atomic file layer; workspace lock is not a hard cross-process OS mutex).
- **Embed-Cache Primary Key**: Embed-cache PK is still `content_hash` only.
- **Doctor Database Repair**: `sil doctor` reports SQLite integrity status via `PRAGMA integrity_check;` but does **not** automatically rebuild or recover a corrupted SQLite database file.
- **Windows File Rename Semantics**: Same-directory atomic rename relies on POSIX `rename()` / `os.replace()`; non-POSIX or Windows filesystem semantics were not tested in this wave.
