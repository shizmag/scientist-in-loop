# PR-C1 — Re-parse without data loss

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **parse-engineer** for scientist-in-loop. Ship ONLY PR-C1.

## Goal

Force re-parse (TUI `E` / `Shift+E`) must **not** delete the source row before parsing. A failed re-parse leaves the previous FTS/index intact. Default CLI parse of an already-parsed file still fails.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §C1, KD-9, KD-10
- Parse: `crates/sil-parse/src/batch.rs` (`parse_one`, `AlreadyParsed`)
- TUI force path: `crates/sil-tui/src/app/jobs.rs` `queue_source_parse` — today `if force { let _ = db.remove_source(&doc_id); }`
- CLI idempotency: `crates/sil/tests/e2e_hardening.rs` `reparse_same_pdf_fails_idempotently` must stay red (failure)
- MCP parse: `crates/sil-mcp/src/tools/mod.rs` — only touch if a force/reparse path exists
- DB: `crates/sil-db` — add a transactional helper only if needed

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. Do **not** invent a CLI `--force` flag just for this PR.
4. Default `parse_one` behavior unchanged (`AlreadyParsed` still errors).
5. Prefer unit tests co-located; clippy clean.

## Requirements

1. Introduce `ParseOptions { allow_reparse: bool }` (default `false`). Keep `parse_one(...)` working via a wrapper or defaulted options so CLI / MCP unparsed paths do not change signatures unnecessarily.
2. When `allow_reparse=true`, skip `AlreadyParsed` rejection. **Never** call `remove_source` before parse.
3. Wrap **all** DB mutations in `parse_one` (`upsert_parsed`, `save_source_references`, chunk inserts, any other writes you find) in **one** SQLite transaction. On error, the previous rows remain.
4. TUI `queue_source_parse`: delete the `db.remove_source` line; pass `allow_reparse=true` when `force` is true.
5. Tests:
   1. Unit: parse once with stub content `first` → force re-parse with a failing runner → `get_source_content` still contains `first`.
   2. Existing e2e `reparse_same_pdf_fails_idempotently` still fails with “already parsed”.
   3. Force re-parse success replaces content (search/content finds the new token).
6. Audit MCP parse handler; if it re-parses by deleting first, apply the same options. If it never force-reparses, leave it.

## Out of scope

- Marker / xberg algorithm changes
- TUI chrome / help keys
- Deleting source files from disk
- Atomic file writes (A2)
- CLI `--force` flag
- Embed-cache PK

## Verify

```bash
cargo test -p sil-parse -p sil-tui
cargo test -p sil --test e2e_hardening
cargo clippy -p sil-parse -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Files changed, `ParseOptions` API, confirmation `remove_source` is gone from the force path, residual risk.
