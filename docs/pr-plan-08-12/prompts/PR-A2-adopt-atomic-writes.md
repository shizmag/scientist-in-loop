# PR-A2 — Adopt atomic writes

Copy the block below into an agent session. **Depends on A1.**

---

## Role

You are a focused Rust **core-engineer** for scientist-in-loop. Ship ONLY PR-A2. Base on A1 (`write_atomic` / `write_atomic_str` already in sil-core).

## Goal

Every durable project/user file write goes through `sil_core::write_atomic` / `write_atomic_str` so a mid-write crash cannot truncate manuscripts, bib, config, or structure.

## Repo context

- Parent plan: `docs/pr-plan-08-12/pr-plan.md` §A2
- Helper: `sil_core::write_atomic` / `write_atomic_str` (A1)
- Known production `fs::write` sites (audit again; do not miss new ones):
  - `crates/sil-tui/src/app/bib_actions.rs` — `references.bib`
  - `crates/sil-tui/src/app/jobs.rs` — hydration bib write
  - `crates/sil-tui/src/app/handlers/mod.rs` — config + draft
  - `crates/sil-mcp/src/tools/mod.rs` — draft edit/todo, bib upsert/promote
  - `crates/sil-core/src/structure.rs` — `Structure::save`
  - `crates/sil-core/src/settings.rs` — `GlobalSettings::save`, `SettingsCache::save`
  - `crates/sil-core/src/workspace_lock.rs` — `write_lock`
  - `crates/sil/src/commands/doctor.rs` — `--fix` bib write
  - `crates/sil-agent/src/estimate.rs` — `write_estimate_report`
  - `crates/sil-latex/src/split_write.rs` — section files

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit.
3. Re-read-before-write bib policy (ADR-010) is unchanged — only the final disk write becomes atomic.
4. Init `write_if_missing` may stay non-atomic (first create, no overwrite of user data).
5. Do **not** rewrite test fixtures that `fs::write` sample YAML into tempdirs unless they go through `save()`.

## Requirements

1. Replace production writes listed above with `write_atomic` / `write_atomic_str`. Map `io::Error` into the existing error type at each site (`SilError`, `StructureError`, `CallToolResult::error`, status string, etc.).
2. TUI sites that currently `let _ = fs::write(...)` must still surface write failure in the status line when they already did; do not newly swallow errors.
3. After the sweep, run:

   ```bash
   rg -n 'fs::write\(' crates/sil-tui/src crates/sil-mcp/src crates/sil-core/src crates/sil-agent/src crates/sil-latex/src crates/sil/src/commands
   ```

   Remaining hits must be tests, init-if-missing, or non-project files. List them in the deliverable with a one-line reason each.
4. Existing unit/e2e tests that save structure/settings/bib/draft must stay green (behavior-preserving).
5. Do not add exclusive locking or `is_busy` checks.

## Out of scope

- Changing `write_atomic` itself (A1)
- Init scaffold first-create writes
- Python `download_pdf.py` (D2)
- SQLite (B1)
- Embed-cache schema

## Verify

```bash
cargo test -p sil-core -p sil-tui -p sil-mcp -p sil-agent -p sil-latex
cargo clippy -p sil-core -p sil-tui -p sil-mcp -p sil-agent -p sil-latex --all-targets -- -D warnings
rg -n 'fs::write\(' crates/sil-tui/src crates/sil-mcp/src crates/sil-core/src crates/sil-agent/src crates/sil-latex/src crates/sil/src/commands
```

## Deliverable

Files changed, leftover `fs::write` inventory with reasons, residual risk.
