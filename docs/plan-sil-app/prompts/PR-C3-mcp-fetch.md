# PR-C3 — MCP sil_sources fetch via sil-app

Copy the block below into an agent session (worktree-isolated if parallel).

---

## Role

You are a focused Rust **MCP engineer** for scientist-in-loop. Ship ONLY PR-C3.

## Goal

Replace `handle_fetch_source` with `sil_app::fetch_source`. MCP fetch **writes official bib** when the resolver succeeds. Surface `parse_error` instead of swallowing parse failures. Hard error only on download failure.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §5.5, §6 C3, KD-11
- Prerequisite: **PR-C1 merged**
- Today: `crates/sil-mcp/src/tools/mod.rs` `handle_fetch_source` (~901)
  - download + optional parse (`if let Ok` swallows errors)
  - JSON: `downloaded_path`, `parsed`, `title`, `source_id`, `commit_proposal`
  - **no** references.bib write
- `sil-mcp` may already depend on `sil-app` if B2 landed; add the dep if missing
- Do **not** put Marker stub content in sil-app. If MCP tests rely on `SIL_MARKER_STUB` for **parse** tool, that is `handle_parse_source`, not fetch. Fetch with `parse=true` and no runner → `parse_error` on the result.

## Shared invariants

1. Match existing Rust style; minimal diff.
2. Never `git commit`.
3. `parse = !no_parse` (default parse on).
4. Download failure → `CallToolResult::error`.
5. Parse failure → success JSON with `parsed=false` and `parse_error` set.

## Requirements

1. `handle_fetch_source`:
   - Validate `target`
   - `AppContext::from_cwd()`
   - `fetch_source(..., FetchSource { target, parse: !no_parse })`
   - Keep existing keys: `downloaded_path`, `parsed` (bool), `title`, `source_id`, `commit_proposal` (same object shape: `proposal_subject`, `proposal_body`, `full_commit_message`, `action_trailer`)
   - Add:
     - `parse_error`: string or null
     - `bib`: null or `{ cite_key, replaced, proposal, path }`
     - `never_committed`: true
     - optional `parse_proposal` if useful; not required for compatibility
2. If `sil-app` is already a dependency from B2, reuse it.
3. Add a unit test if cheap: missing `target` still errors (today’s behavior). Do not add network fetch tests.
4. Update any existing fetch tests if JSON assertions break — extras are additive except `parsed` meaning stays bool.

## Out of scope

- CLI / TUI
- Changing `handle_parse_source`
- Splitting `tools/mod.rs`
- STAGES / ADR

## Verify

```bash
cargo test -p sil-mcp
cargo clippy -p sil-mcp --all-targets -- -D warnings
cargo fmt --all -- --check
```

## Deliverable

JSON before/after, confirmation download errors vs parse errors.
