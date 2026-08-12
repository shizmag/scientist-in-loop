# PR-A1 — Atomic write primitive

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **core-engineer** for scientist-in-loop. Ship ONLY PR-A1.

## Goal

Add a shared crash-safe write helper in `sil-core`. A killed process must leave either the previous complete file or a leftover temp — never a truncated destination.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §A1, KD-3, KD-4, KD-5
- New module: `crates/sil-core/src/atomic.rs`; export from `crates/sil-core/src/lib.rs`
- Today every durable writer uses `std::fs::write` (call-site migration is **A2**, not this PR)

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Never auto-commit.
3. No extra crates for Windows replace.
4. Prefer unit tests co-located; clippy clean on touched crates.

## Requirements

1. Add `sil_core::write_atomic(path: &Utf8Path, bytes: &[u8]) -> io::Result<()>`.
2. Add `sil_core::write_atomic_str(path: &Utf8Path, text: &str) -> io::Result<()>` (UTF-8 wrapper).
3. Create parent directories if missing.
4. Write a temp file in the **same directory** as the destination. Temp name: `.{filename}.{pid}.{uniq}.tmp` (uniq = nanos or similar; must not collide on two sequential writes).
5. `File::sync_all` the temp, then `fs::rename` onto the destination.
6. On any error after the temp is created, best-effort `remove_file` the temp.
7. On POSIX, a successful return means the destination contains the complete new bytes (or, if rename never happened, the complete old bytes).
8. Unit tests (no extra crates):
   1. Write + read-back.
   2. Overwrite existing file; destination is never a mix of old and new after a successful return.
   3. Failed write (e.g. parent path is a file) leaves an existing destination unchanged.
   4. Temp naming includes pid; two sequential writes both succeed.
9. `#![deny(missing_docs)]` — document the public functions.
10. Do **not** migrate production call sites (A2).

## Out of scope

- Migrating TUI / MCP / CLI / settings / bib writers (A2)
- Windows-special crate or `MoveFileEx`
- Exclusive flock / honoring `is_busy`
- Changing gitignore (unless you introduce a committed temp, which you must not)

## Verify

```bash
cargo test -p sil-core
cargo clippy -p sil-core --all-targets -- -D warnings
```

## Deliverable

Files changed, API signatures, residual Windows rename risk note.
