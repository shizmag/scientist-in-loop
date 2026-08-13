# PR-D1 — Last `references.bib` writers through richest policy

Copy the block below into an agent session.

---

## Role

You are a focused Rust engineer for scientist-in-loop. Ship ONLY PR-D1.

## Goal

Close the leftover bibliography writers that Stage 12 did not route through richest upsert policy. After this PR, **TUI user actions and TUI hydration** write `references.bib` only via `sil_app::upsert_bib`. Doctor `--fix` cannot depend on `sil-app` (cycle: `sil-app` → `sil-parse`); align it on `upsert_bib_entry_with_options { preserve_cite_key: true }` instead.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` KD-4, KD-5, KD-6, KD-13 (KD-13 is hereby **superseded** for hydration)
- ADR residual to close: `docs/adr/ADR-014-sil-app-usecase-layer.md` §Residuals “TUI Hydration Apply”
- **Gap A (unplanned leftover):** `crates/sil-tui/src/app/handlers/mod.rs` ~398–447 — References tab `p` inlines `mark_tui_added` + `upsert_bib_entry` (no `preserve_cite_key`). `P` already calls `promote_selected_bib_entry()`.
- **Gap B (planned residual):** `crates/sil-tui/src/app/jobs.rs` `poll_background_hydration` ~637–690 — on `HydrationOutcome::Success` reads bib, preserves tui-added marker, `upsert_bib_entry_with_options(preserve_cite_key: true)`, `write_atomic_str`.
- **Gap C (cycle-safe policy):** `crates/sil-parse/src/checkers/mod.rs` ~266–275 — `--fix` / `autofix` uses `upsert_bib_entry` (preserve **false**). Caller later persists `report.updated_bib_content`. **Do not** add `sil-app` to `sil-parse`.
- Already correct: `crates/sil-tui/src/app/bib_actions.rs` append/promote (Stage 12 B3). Copy that pattern.

Hydration marker rule (keep this product behavior):

- If an existing `references.bib` block is the same paper **and** still has `% [sil: tui-added]`, upsert official bib with `draft=true`.
- Otherwise `draft=false`.
- Do **not** invent a new `sil-app` flag. Compute `draft` in the adapter, then call `upsert_bib`.

## Shared invariants

1. Match existing Rust style; minimal diff; no drive-by refactors.
2. Never auto-commit / never `git commit`.
3. `upsert_bib` always `preserve_cite_key: true`. Do not call `upsert_bib_entry` from TUI.
4. TUI appends from the user are `draft=true`. Hydration uses the marker rule above.
5. `sil-parse` must not depend on `sil-app`.
6. Do not change search / rank / fetch / cite CLI.
7. Prefer unit tests co-located; clippy `-D warnings` on touched crates.

## Requirements

### A — References tab `p`

1. Move the `KeyCode::Char('p')` References-tab write out of `handlers/mod.rs`.
2. Add (or reuse) a `bib_actions.rs` method, e.g. `append_selected_extracted_refs_to_bib`, that:
   - Uses the same selection rules as today (`marked_ref_ids` else selected `filtered_source_references()`).
   - For each entry: `e.to_bibtex()` then `sil_app::upsert_bib(draft=true)` (do **not** pre-mark; use-case marks).
   - Sequential upserts (each re-reads disk).
   - Then `load_project_references_bib()`, clear marks, `queue_ref_hydration` when `should_attempt_metadata_fetch`, same status strings.
3. Handler becomes a one-liner: `self.append_selected_extracted_refs_to_bib()`.
4. Do not change the `p` keybinding or help text unless a string is now wrong.

### B — Hydration apply

1. In `poll_background_hydration` success path, replace the manual read/parse/upsert/`write_atomic_str` with:
   - `AppContext::from_root`
   - Determine `draft` from current bib + `is_same_paper` + `is_tui_added_bib_block` (same scan as today)
   - `sil_app::upsert_bib(UpsertBib { entry: official_bib, draft })`
2. Keep job chrome: success/fail `JobOutcome`, retry payload, batch counters, `load_project_references_bib` on success.
3. Keep `catch_unwind` on workers. Do not change hydrate **queue** / network resolve.
4. Update `hydration_tests.rs` if they assert direct `upsert_bib_entry_with_options` in the apply path.

### C — Checker autofix (no sil-app)

1. In `crates/sil-parse/src/checkers/mod.rs`, replace `upsert_bib_entry` with `upsert_bib_entry_with_options(..., UpsertOptions { preserve_cite_key: true })`.
2. Do not add a `sil-app` dependency.
3. Add or adjust a unit test that autofix of a same-DOI official entry **keeps** the existing cite key.

### D — Docs honesty

1. `docs/adr/ADR-014-sil-app-usecase-layer.md`: remove or rewrite the “TUI Hydration Apply” residual. Mention handlers `p` is now via `sil-app`. Leave search/rank residual. Optionally note checker `--fix` is policy-aligned but still in `sil-parse` (cannot import `sil-app`).
2. `STAGES.md` Stage 12 residual sentence: drop “TUI hydration apply still updates `references.bib` directly”; mention `--fix` stays in `sil-parse` if you still want that residual listed.

## Out of scope

- Search / rank unification
- New `sil-app` APIs (`preserve_existing_draft_marker`, batch upsert)
- Changing MCP/CLI cite or fetch
- Workspace lock
- Splitting `handlers/mod.rs` or `jobs.rs` beyond the write-path change
- Python download / parse pipeline

## Verify

```bash
cargo test -p sil-tui
cargo test -p sil-parse
cargo test -p sil-app
cargo clippy -p sil-tui -p sil-parse --all-targets -- -D warnings
cargo fmt --all -- --check
```

Grep gate (must be empty in TUI except comments / tests if any):

```bash
rg -n "upsert_bib_entry" crates/sil-tui/src
```

Expected: no production hits in `handlers/mod.rs` or `jobs.rs`. `sil-app` / `sil-core` / `sil-parse` may still call the primitive.

## Deliverable

Files changed; confirmation TUI has zero `upsert_bib_entry` production calls; how hydration computes `draft`; checker cite-key test name; ADR residual edit.
