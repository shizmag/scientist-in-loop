# PR-B1 — SQLite WAL + busy_timeout + integrity

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **db-engineer** for scientist-in-loop. Ship ONLY PR-B1.

## Goal

Every `SilDb` open uses WAL + a 5s busy timeout so TUI worker threads, CLI, and MCP do not fail immediately with `SQLITE_BUSY`. Doctor reports `PRAGMA integrity_check` (read-only).

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §B1, KD-6, KD-7, KD-8, KD-19
- Open path: `crates/sil-db/src/lib.rs` (`SilDb::open`, `open_in_memory`)
- Schema/pragmas: `crates/sil-db/src/schema.rs` (already `PRAGMA foreign_keys = ON`)
- Doctor: `crates/sil/src/commands/doctor.rs` (check shape: `name`, `ok`, `detail`; optional `extra`)
- Init gitignore: templates / `crates/sil/src` init gitignore writer

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. **Do not** change `embedding_cache` PRIMARY KEY (explicit residual).
4. **Do not** add FTS rebuild or doctor `--fix` database repair.
5. Prefer unit tests co-located; clippy clean.

## Requirements

1. On every `SilDb::open` and `open_in_memory`, after connect and with/before migrate, apply:

   ```
   PRAGMA journal_mode = WAL;
   PRAGMA busy_timeout = 5000;
   PRAGMA foreign_keys = ON;
   PRAGMA synchronous = NORMAL;
   ```

   Prefer a single helper so both open paths cannot drift.

2. File-backed DBs must report `PRAGMA journal_mode` = `wal`. In-memory may stay `memory` — document this and **do not fail** a test that opens `open_in_memory`.

3. Unit test: two `SilDb::open` on the same temp file; overlapping writer + reader must not return `SQLITE_BUSY` / `database is locked` immediately (busy_timeout must absorb a short lock).

4. Doctor: new check named `sqlite integrity` (or equivalent stable name). Run `PRAGMA integrity_check;` (or `integrity_check(1)`). `ok` iff the result is `ok`. No rebuild, no delete, no dump.

5. If init project `.gitignore` / templates do not ignore `*.db-wal` and `*.db-shm`, add them. Do not commit sidecar files.

6. Existing sil-db / sil tests stay green.

## Out of scope

- Embed-cache composite PK / dimension key
- FTS rebuild / `REINDEX`
- Changing schema tables beyond pragmas
- Exclusive file locks
- Atomic file writes (A1/A2)

## Verify

```bash
cargo test -p sil-db -p sil
cargo clippy -p sil-db --all-targets -- -D warnings
```

## Deliverable

Files changed, where pragmas run, doctor check name, gitignore note, residual (in-memory journal_mode, embed-cache PK still wrong).
