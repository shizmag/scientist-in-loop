# PR-W3 — Grounding modal (slip-ok)

Copy the block below into an agent session. **After R4.** This PR is **slip-ok**.

---

## Role

You are the **grounding engineer** for scientist-in-loop. Ship ONLY PR-W3.

## Goal

Show ranked sources that might support the current draft section (or selected idea). Display-only. Inserting a cite is the existing R4 command. Do not write the draft from this modal.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.14, KD-22
- Today: MCP `sil_cite action=ground` / CLI-adjacent helpers exist. Find the library function (likely `sil-app` or `sil-agent` / search) and **call it**. Do not reimplement RAG.

## Shared invariants

1. Minimal diff. Display only.
2. Never auto-commit. Never insert `\cite` from this modal.
3. Register `GroundSection` on the palette.
4. Clippy clean.

## Requirements

1. Command uses current draft section body (or first N chars) as the query.
2. Modal lists ranked hits (title, score, source id). Esc closes.
3. Enter on a row may **select** that source (switch to Sources / status) but must not write tex.
4. Unit tests:
   1. Opening the modal does not change `paper_draft.tex`.
   2. Empty search results → empty-state line, no panic.
   3. If the helper needs a DB, use a fixture; skip network.

## Out of scope

- Auto-insert cites
- New MCP tool
- Editing the draft

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Which helper is called, modal fields, proof of no write.
