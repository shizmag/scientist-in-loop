# PR-Z — STAGES Stage 12 + ADR-014 + docs honesty

Copy the block below into an agent session.

---

## Role

You are a focused **docs engineer** for scientist-in-loop. Ship ONLY PR-Z.

## Goal

Record Stage 12 honestly: CLI / TUI / MCP share `sil-app` for bib upsert/promote and fetch. Residual: search/rank drift, TUI hydration still a second bib writer.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-sil-app/pr-plan.md` §6 Z, §8, §9, KD-15
- Prerequisite: **B1–B3 and C1–C4 merged** (or note any not-landed PR as residual)
- Prior ADR style: `docs/adr/ADR-013-crash-safe-robustness.md`
- `STAGES.md` last stage is Stage 11 (crash-safe)
- README may claim fetch / cite / MCP behavior — grep and align

## Shared invariants

1. Docs honesty: do not claim search/rank are unified. Do not claim hydration goes through `sil-app`.
2. Do not invent product features.
3. No new top-level project directories.
4. Minimal README edits — only claims that are now false.

## Requirements

1. `STAGES.md` — add **Stage 12** ✅:
   - `sil-app` use-case crate
   - `upsert_bib` / `promote_bib` / `fetch_source`
   - CLI + MCP + TUI adapters for those three
   - Richest policy (preserve cite key, official bib on fetch, TUI fetch does not parse)
   - Residual sentence: search still FTS-only on CLI; rank embedder settings still differ; TUI hydration apply still writes bib directly
   - Point at `docs/plan-sil-app/` and `docs/adr/ADR-014-sil-app-usecase-layer.md`
2. New `docs/adr/ADR-014-sil-app-usecase-layer.md`:
   - Status: Accepted (Stage 12)
   - Context: three-surface drift; `sil` binary cycle
   - Decision: `sil-app` crate; richest unification; role flags `draft` / `parse`; always `preserve_cite_key`; fetch resolver order
   - Consequences: MCP fetch writes bib; CLI append preserves keys; MCP `preserve_cite_key: false` ignored
   - Residuals: hydration writer; search/rank; URL+TUI-no-parse may skip bib
3. README / agent skills / MCP docs:
   - Grep for “19 tools”, wrong fetch semantics, “CLI search hybrid”, tool counts
   - If README says MCP fetch only downloads, update: it now upserts official bib when resolved
   - Do not rewrite the whole README
4. Do **not** change product code except a comment if a public rustdoc in `sil-app` is missing — prefer not.

## Out of scope

- Implementing missing B/C PRs
- Search / rank wave
- Hydration rewrite
- New MCP tools

## Verify

```bash
# no code required; sanity:
test -f docs/adr/ADR-014-sil-app-usecase-layer.md
rg -n "Stage 12" STAGES.md
```

Manual: read STAGES Stage 12 and ADR-014 once for honesty vs the tree.

## Deliverable

Files changed, residual list copied into ADR, any README lines edited.
