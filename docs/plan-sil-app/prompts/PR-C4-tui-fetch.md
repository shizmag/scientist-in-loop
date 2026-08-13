# PR-C4 — TUI fetch job via sil-app (parse=false)

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **TUI engineer** for scientist-in-loop. Ship ONLY PR-C4.

## Goal

Run `sil_app::fetch_source(parse=false)` on the existing fetch worker thread so TUI fetch downloads **and** upserts official bib when the richest resolver can, without parsing. Keep the empty DB stub and hydration jobs as they are.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.5, §6 C4, KD-9, KD-10, KD-12, KD-13
- Prerequisite: **PR-C1 merged**
- Today: `crates/sil-tui/src/app/jobs.rs`
  - `queue_source_fetch` (~244): worker calls `fetch_source_target` only
  - `poll_background_fetch` (~411): on success, `upsert_parsed(&doc, "")` then `reload_sources`
- `sil-tui` may already depend on `sil-app` if B3 landed; add the dep if missing
- Honest limit (KD-10): URL target + no parse often yields `bib=None`. DOI/arXiv targets should write bib.

## Shared invariants

1. Match existing Rust style; minimal diff.
2. **`parse=false` always** on this job. Do not start a parse from fetch.
3. Keep empty `upsert_parsed("",…)` in the **adapter** (poll), not by changing sil-app.
4. Do **not** change hydration apply (`HydrationOutcome::Success` block).
5. Keep `catch_unwind` isolation on the worker.
6. Never auto-commit.

## Requirements

1. Worker body: `AppContext::from_root(root)` + `fetch_source(FetchSource { target, parse: false })`.
2. `FetchJobResult` (in `types.rs` if needed):
   - Keep `target`, `label`, `duration_ms`
   - `result: Result<...>` should carry at least downloaded path; add optional bib cite_key / replaced so the status line can say “fetched + bibliography updated”
   - On use-case `Err`, treat as fetch failure (retry payload `RetryPayload::Fetch` unchanged)
3. `poll_background_fetch` success path:
   - Keep empty stub upsert + `reload_sources`
   - If bib was written: `load_project_references_bib()` (or equivalent already used after append)
   - Status: `✓ Fetched source '…'` and mention bib if present
4. Do not auto-queue parse or hydration from fetch completion.
5. Update TUI tests if they mock `fetch_source_target` directly.

## Out of scope

- Hydration apply (KD-13)
- Parse-on-fetch
- CLI / MCP
- Job-channel unification
- STAGES / ADR

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

Worker call signature (`parse=false`), `FetchJobResult` field changes, confirmation hydration apply untouched.
