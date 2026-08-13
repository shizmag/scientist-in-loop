# PR-A1 — sil-app crate + upsert_bib / promote_bib

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **library engineer** for scientist-in-loop. Ship ONLY PR-A1.

## Goal

Add crate `sil-app` with `AppContext`, `AppError`, `upsert_bib`, and `promote_bib`. No CLI / TUI / MCP wiring.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.1–5.3, KD-1–KD-8, KD-14
- Today `crates/sil` is binary-only and depends on `sil-tui` + `sil-mcp` (cycle if they depend back)
- Bib string logic already lives in `sil_core::bib` (`upsert_bib_entry_with_options`, `mark_tui_added_bib_entry`, `unmark_tui_added_bib_entry`, `is_same_paper`, `parse_bib_blocks`, `extract_bib_entry_info`)
- Proposals: `sil_git::proposal_for_action` + `SciAction::UpdateBibliography` / `PromoteBibliography`
- Atomic write: `sil_core::write_atomic_str`
- References path: `sil_core::paths::rel::REFERENCES`

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Never auto-commit. Never call `git`.
3. `preserve_cite_key` is **always true** inside `upsert_bib` (KD-5). No option on the request struct.
4. Use-cases are sync, no `SilUi`, no JSON, no Ratatui.
5. `#![deny(missing_docs)]` like `sil-core` / `sil-agent`.
6. Prefer unit tests co-located; clippy clean on `sil-app`.

## Requirements

1. Workspace:
   - Add `"crates/sil-app"` to `[workspace].members` in root `Cargo.toml`.
   - Add `sil-app = { path = "crates/sil-app" }` to `[workspace.dependencies]`.
2. `crates/sil-app/Cargo.toml`:
   - `publish = false`
   - Deps: `sil-core`, `sil-git`, `thiserror`, `camino` (workspace). Dev: `tempfile`.
   - **Do not** depend on `sil-parse`, `sil-db`, `sil-tui`, `sil-mcp`, `sil`.
3. Modules:
   - `src/lib.rs` — re-export public API
   - `src/error.rs` — `AppError` via `thiserror` (at least: not-in-project, io, invalid bib, not found)
   - `src/context.rs` — `AppContext { root, paths, config }`
   - `src/bib.rs` — upsert / promote
4. `AppContext`:
   - `from_root(root)` — `ProjectPaths::new`, `Config::load(&paths.config()).unwrap_or_default()`
   - `from_cwd()` — `project_root_from_cwd()` then `from_root`
   - Do **not** open SQLite.
5. `upsert_bib(ctx, UpsertBib { entry, draft }) -> Result<UpsertBibResult, AppError>`:
   1. Reject empty / whitespace-only `entry`.
   2. Reject if `entry` contains no `@` (same as MCP: “not valid BibTeX”).
   3. If `draft`, `mark_tui_added_bib_entry`.
   4. Re-read `references.bib` (`""` if missing).
   5. `upsert_bib_entry_with_options(..., UpsertOptions { preserve_cite_key: true })`.
   6. Resolve cite key: scan updated blocks with `is_same_paper` against the incoming entry (same as `handle_upsert_bib` in `crates/sil-mcp/src/tools/mod.rs`).
   7. `write_atomic_str`.
   8. `proposal_for_action(SciAction::UpdateBibliography, Some("Update bibliography: {cite_key}"), Some(body))`.
   9. Return `{ cite_key, replaced, path, draft, proposal }`.
6. `promote_bib(ctx, PromoteBib { target }) -> Result<PromoteBibResult, AppError>`:
   1. Missing `references.bib` → error.
   2. Re-read. Build `BibEntryInfo` with cite_key/title/doi/arxiv_id all `Some(target)` (same CLI/MCP hack).
   3. First block where `is_same_paper` **or** cite-key eq_ignore_ascii_case: `unmark_tui_added_bib_entry`; record `had_marker`.
   4. No match → error.
   5. Join blocks with `\n\n` + trailing `\n` (same as MCP).
   6. `write_atomic_str`. `proposal_for_action(PromoteBibliography, ...)`.
   7. Return `{ cite_key, had_marker, path, proposal }`.
7. Unit tests (temp dir that looks enough like a sil project: create `.sil/` so `from_root` works; config optional):
   1. Upsert new entry → file contains `@article{...}`; `replaced == false`; proposal contains `Sci-Action: update-bibliography`.
   2. Upsert same paper (same DOI) with a **new** cite key → existing key **preserved**; `replaced == true`.
   3. `draft=true` writes `% [sil: tui-added]`.
   4. Promote strips the marker; `had_marker == true`.
   5. Promote unknown target errors.
   6. Empty / non-BibTeX entry errors.
   7. `from_cwd` from a temp dir **without** a sil project errors.
8. Tests must not create a git commit. Assert working tree / no `git` invocation.

## Out of scope

- Wiring CLI / TUI / MCP (B1–B3)
- `fetch_source` (C1)
- Depending on `sil-parse` / `sil-db`
- Changing `sil_core::bib` algorithms
- Search / rank / hydration
- STAGES / ADR (Z)

## Verify

```bash
cargo test -p sil-app
cargo clippy -p sil-app --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Files changed, public signatures, residual risk note (none expected beyond “adapters not switched yet”).
