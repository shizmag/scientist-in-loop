# PR-B3 — TUI explicit bib actions via sil-app

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **TUI engineer** for scientist-in-loop. Ship ONLY PR-B3.

## Goal

Route explicit TUI bibliography writes through `sil-app::upsert_bib` / `promote_bib`. Leave hydration jobs in `jobs.rs` **untouched**.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.5, §6 B3, KD-4, KD-13
- Prerequisite: **PR-A1 merged**
- Today: `crates/sil-tui/src/app/bib_actions.rs`
  - `append_selected_source_to_bib` — `suggest_from_source` + `mark_tui_added` + `upsert_bib_entry`
  - `append_selected_viewing_ref_to_bib` / `append_all_viewing_refs_to_bib` — `to_bibtex` + mark + upsert loop
  - `promote_selected_bib_entry` — unmark + rewrite blocks (`is_same_paper` only)
- After append, TUI queues hydration (`queue_source_hydration` / `queue_ref_hydration`). **Keep that.**
- Hydration **apply** in `jobs.rs` (~608+) still uses `upsert_bib_entry_with_options` — **do not change**.

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never auto-commit. Do not print/show git proposal in the status line unless already doing something similar (today: ✓ status only — keep that).
3. TUI appends are `draft=true` (role flag).
4. Do **not** edit `jobs.rs` hydration apply.
5. After writes: `load_project_references_bib()` as today.

## Requirements

1. Add `sil-app` to `crates/sil-tui/Cargo.toml`.
2. Build `AppContext::from_root(project_root)` when a project is loaded.
3. `append_selected_source_to_bib`:
   - Build local bib via `suggest_from_source` (unchanged).
   - `upsert_bib(draft=true)` (use-case applies the tui-added marker — do not double-mark).
   - Then existing hydration queue + status messages.
4. Viewing-ref append / append-all:
   - Each entry: `e.to_bibtex()` then `upsert_bib(draft=true)`.
   - Multiple entries: sequential `upsert_bib` calls (each re-reads disk — correct).
   - Keep marked-id / fetch_count / status behavior.
5. `promote_selected_bib_entry`:
   - Resolve cite key from the selected block.
   - Early-out if not tui-added may remain as UX (status “already promoted”) **or** just call `promote_bib` (idempotent unmark). Prefer calling `promote_bib` with the cite key so matching includes DOI/title, not only `is_same_paper` on the selected block.
   - Status ✓ / error from the result.
6. Update TUI unit tests if they stub/assert direct `upsert_bib_entry` calls.
7. Do not change keybindings or help text unless a string is now wrong.

## Out of scope

- `jobs.rs` hydration apply (KD-13)
- `queue_source_fetch` (C4)
- CLI / MCP
- Search / rank
- STAGES / ADR

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Files changed, confirmation `jobs.rs` hydration apply is untouched, how multi-ref append calls `upsert_bib`.
