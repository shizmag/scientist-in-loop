# PR-B2 — MCP sil_cite upsert / promote via sil-app

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **MCP engineer** for scientist-in-loop. Ship ONLY PR-B2.

## Goal

Make `handle_upsert_bib` and `handle_promote_bib` thin adapters over `sil-app`. Keep JSON keys so existing MCP unit tests stay green. Document KD-5: `preserve_cite_key: false` is ignored.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.5, §6 B2, KD-5, KD-7
- Prerequisite: **PR-A1 merged**
- Today: `crates/sil-mcp/src/tools/mod.rs` `handle_upsert_bib` (~981) and `handle_promote_bib` (~1054)
- Tests: same file ~2922–3058 (`test_upsert_bib_*`, `test_promote_bib_*`)
- Schema still has `preserve_cite_key` (keep it so old agents do not break)

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never `git commit`. Keep `never_committed: true`.
3. Use-case always `preserve_cite_key: true`. Do not pass a preserve flag into `sil-app`.
4. Do not change `sil_cite` suggest / ground handlers.
5. Prefer keeping `handle_upsert_bib` / `handle_promote_bib` function names (tests call them directly).

## Requirements

1. Add `sil-app` to `crates/sil-mcp/Cargo.toml`.
2. `handle_upsert_bib`:
   - Keep arg validation that is MCP-specific **or** let `sil-app` validate empty / missing `@` and map `AppError` → `CallToolResult::error` with similar wording so existing tests still match (`empty`, `not valid BibTeX`, `Missing required parameter: entry`).
   - `AppContext::from_cwd()` (same as today’s `get_project_paths`).
   - `upsert_bib(..., UpsertBib { entry, draft })`.
   - JSON **must** still include: `wrote`, `cite_key`, `replaced`, `path`, `draft`, `proposal` (message string), `never_committed: true`.
3. `handle_promote_bib`:
   - Map to `promote_bib`.
   - JSON: `wrote`, `cite_key`, `replaced` (= `had_marker`), `path`, `proposal`, `never_committed: true`.
4. Update `test_upsert_bib_preserve_cite_key` if needed (still asserts old key kept).
5. **Add or rewrite** a test that `preserve_cite_key: false` **still preserves** the existing key (KD-5). Name it so the policy is obvious, e.g. `test_upsert_bib_preserve_cite_key_false_is_ignored`.
6. Existing never-commit HEAD tests stay green.

## Out of scope

- CLI / TUI
- `sil_sources` fetch
- Splitting `tools/mod.rs`
- Changing suggest / ground
- STAGES / ADR

## Verify

```bash
cargo test -p sil-mcp
cargo clippy -p sil-mcp --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Files changed, JSON compatibility note, KD-5 test name.
