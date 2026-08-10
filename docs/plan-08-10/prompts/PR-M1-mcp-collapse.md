# PR-M1 — Collapse MCP tools to 6 max-value endpoints

Parent design: [../pr-plan.md](../pr-plan.md).

## Role

Ship ONLY PR-M1. See parent plan for full design, KD-1..KD-9, and the normative migration table.

## Goal

Reduce registered MCP tools from **19** to **6** by grouping existing handlers under action-dispatched (or flag-dispatched) tools. Preserve all prior capabilities and governance (**never auto-commit**).

## Target surface (normative)

| Tool | Dispatch | Must preserve behavior of |
|------|----------|---------------------------|
| `sil_context` | flags + optional `skill` / `list_skills` | `sil_get_workspace_context`, `sil_list_skills`, `sil_invoke_skill`, structure **read**, `sil_list_todos` |
| `sil_sources` | `action`: `search` \| `get` \| `fetch` \| `parse` \| `rank` | `sil_search_sources`, `sil_get_source_context`, `sil_fetch_source`, `sil_parse_source`, `sil_rank_draft` |
| `sil_cite` | `action`: `suggest` \| `ground` \| `upsert` \| `promote` | `sil_suggest_citations`, `sil_ground_claims`, `sil_upsert_bib`, `sil_promote_bib` |
| `sil_draft` | `action`: `edit` \| `todo` \| `structure` | `sil_edit_section`, `sil_update_todo`, structure **update** |
| `sil_review` | `action`: `estimate` \| `build` | `sil_estimate_paper`, `sil_build_and_doctor` |
| `sil_propose` | `message` / Sci-Action (same as today) | `sil_propose_commit` |

### Full old → new map

| Old | New |
|-----|-----|
| `sil_get_workspace_context` | `sil_context` |
| `sil_list_skills` | `sil_context` (`list_skills`) |
| `sil_invoke_skill` | `sil_context` (`skill` + `input`) |
| `sil_list_todos` | `sil_context` (todo filters / include) |
| `sil_get_structure` read | `sil_context` |
| `sil_search_sources` | `sil_sources` `action=search` |
| `sil_get_source_context` | `sil_sources` `action=get` |
| `sil_fetch_source` | `sil_sources` `action=fetch` |
| `sil_parse_source` | `sil_sources` `action=parse` |
| `sil_rank_draft` | `sil_sources` `action=rank` |
| `sil_suggest_citations` | `sil_cite` `action=suggest` |
| `sil_ground_claims` | `sil_cite` `action=ground` |
| `sil_upsert_bib` | `sil_cite` `action=upsert` |
| `sil_promote_bib` | `sil_cite` `action=promote` |
| `sil_edit_section` | `sil_draft` `action=edit` |
| `sil_update_todo` | `sil_draft` `action=todo` |
| `sil_get_structure` update | `sil_draft` `action=structure` |
| `sil_estimate_paper` | `sil_review` `action=estimate` |
| `sil_build_and_doctor` | `sil_review` `action=build` |
| `sil_propose_commit` | `sil_propose` |

## Requirements

1. **Exactly 6 tools** in `list_tools()` (`crates/sil-mcp/src/tools/mod.rs`). Names must match the table above.
2. **`call_tool`** dispatches by new name, then `action` / flags, into **existing** `handle_*` functions. Prefer thin wrappers; do not rewrite RAG, bib, estimate, or edit logic.
3. **Hard cut (KD-2):** remove old tool names from registration and match arms. **No alias layer.**
4. **Per-action validation:** if required fields for an action are missing, return a clear MCP error (e.g. `action=search requires query`). See parent plan §4.3 for required-field matrix.
5. **JSON response shape** stays compatible where possible (`never_committed`, `proposal`, Sci-Action fields).
6. **Tests:**
   - Update `server.rs` list-tools test: count **6**, expected names = the six new tools (include Stage 9 tools that were missing from the partial expected list: estimate/edit/ground are folded into `sil_review` / `sil_draft` / `sil_cite`).
   - Rewrite tool-call tests that used old names (grep the crate for `sil_search_sources`, `sil_upsert_bib`, `sil_edit_section`, etc.).
   - Keep **HEAD unchanged** asserts for bib upsert/promote and draft edit paths.
   - Estimate still must not write `paper_draft.tex`.
7. **Docs honesty (in this PR):**
   - `README.md` MCP section: tool count **6**, describe the six tools, include **migration table** (or point to this plan).
   - `STAGES.md`: note Stage 10 MCP collapse; fix any stale 19/22 counts.
   - Sweep `templates/agent/skills/*.md` if they name old MCP tools.
   - If ADRs hard-code tool lists/counts, update or add a short note (prefer minimal ADR edit; full new ADR optional).
8. Match existing Rust style; minimal diff; clippy clean on touched crates.
9. Follow Key Decisions KD-1..KD-9 in the parent plan.

## Invariants (do not break)

- Never auto-commit.
- Estimate never writes `paper_draft.tex` (optional write only under `.sil/reviews/`).
- ONNX / hash fallback wording stays honest in tool descriptions.
- No new top-level project directories; no shell / `run_cli` MCP tool.
- Sci-Action proposals remain the only git-facing surface (`sil_propose` + write-tool proposals).

## Out of scope

- Alias layer / dual registration of old names.
- TUI job chrome, CLI subcommand redesign, ONNX model work, golden dataset F1, GitHub Releases, install/Windows.
- New agent capabilities beyond re-exposing existing handlers.
- Changing ranking algorithms, bib completeness rules, or estimate scoring.

## Implementation hints

- Primary file: `crates/sil-mcp/src/tools/mod.rs` (`list_tools`, `call_tool`, tests module).
- Server tests: `crates/sil-mcp/src/server.rs` (`assert_eq!(tools.len(), 19)` → `6`).
- Reuse property descriptions from the old `Tool` definitions when building union schemas.
- For `sil_context`, a clean design is: always-capable snapshot flags + optional `skill` to invoke + `list_skills` boolean; structure/todo list can ride in the snapshot when include flags are true (matching current `sil_get_workspace_context` defaults).

## Verify

```bash
cargo test -p sil-mcp
cargo test --workspace
cargo clippy -p sil-mcp --all-targets -- -D warnings
```

Docs spot-check:

```bash
rg -n 'list_tools|22 tools|19 MCP|sil_search_sources|sil_propose_commit' README.md STAGES.md crates/sil-mcp
```

Expect: code only exposes the six new names; README claims **6** tools; migration may mention old names in a table.

## Deliverable

1. Six tools registered and fully wired.
2. Tests green; HEAD-stability tests retained for writes.
3. README + STAGES (and skills if needed) honest about count + migration.
4. Short residual risk note: agent prompts / external mcp clients that hard-coded old tool names will break until updated (intentional hard cut).
