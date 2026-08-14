# PR-T6 — Honest lock (PID liveness + confirm)

Copy the block below into an agent session. **After T2.**

---

## Role

You are the **lock engineer** for scientist-in-loop. Ship ONLY PR-T6.

## Goal

Make `.sil/workspace.lock` visible and honest. The TUI takes a session lock, clears it on quit, treats dead PIDs as stale, and asks for confirm before mutating if another **live** holder exists. This is **not** `flock` and not NFS-safe.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.11, KD-15
- Today: `crates/sil-core/src/workspace_lock.rs` is advisory last-writer-wins. `WorkspaceLock { holder, pid, started, op }`. TUI does not consistently claim/clear.
- MCP/CLI may already write the lock in some paths — do not break writers; add liveness helper they can share.

## Shared invariants

1. Minimal diff. No `flock`, no claims of exclusive correctness.
2. Never auto-commit.
3. Last writer still wins at the filesystem layer if the user **confirms**.
4. Clippy clean.

## Requirements

1. `sil_core`: `fn pid_is_alive(pid: u32) -> bool` (Unix `kill(pid, 0)`; Windows best-effort or “assume alive” documented).
2. `fn take_or_stale(paths, new_lock) -> Result<TakeLock>`:
   - missing → write
   - present + dead pid → clear + write
   - present + live other holder → `Held { lock }`
   - present + we already hold → refresh op optional
3. TUI start (project mode): `write_lock(holder=tui, op=session)`.
4. TUI quit (normal `q` path): `clear_lock` if holder is us. Panic hook: best-effort clear (do not make this a science project).
5. Before mutating commands (save, bib upsert, note insert, delete source): if `Held` by other live pid → banner `"{holder} is {op} (pid N)"` + confirm to proceed. Use `UserError` code `lock.held` for the title.
6. Unit tests:
   1. Stale lock with a definitely-dead pid (e.g. 2^31-1 or 1 if not alive) is taken.
   2. Lock with current pid + holder `"mcp"` is `Held` (simulate).
   3. TUI test: mutating dispatch without confirm does not write when Held (inject lock file in a temp project).

## Out of scope

- Hard OS mutex / NFS
- MCP forced to wait (MCP may still write; TUI warns)
- Changing advisory file format beyond optional fields

## Verify

```bash
cargo test -p sil-core -p sil-tui
cargo clippy -p sil-core -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Liveness helper, TUI claim/clear points, confirm flow, residual “MCP can still write”.
