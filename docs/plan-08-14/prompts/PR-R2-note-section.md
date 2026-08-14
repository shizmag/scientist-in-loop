# PR-R2 — Note section picker

Copy the block below into an agent session (worktree-isolated if parallel with R1).

---

## Role

You are the **reader-note engineer** for scientist-in-loop. Ship ONLY PR-R2.

## Goal

After the scientist types a reader note (`n`), pick a draft section (or “End of draft”) so the `# -- X -- #` block lands in the right place. `sil_latex::update_or_insert_idea_block` **already honors `section_id`** — this PR is UI + wiring.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.4, KD-8
- Today: C2 `ModalCaptureNote` inserts at end unless `section_id` is set. Tags: `from-source`, `from: <filename>`, `author_type: human`.
- Sections: `sil_latex` draft split / `\section{...}` titles. `paper_sections` is already on `App`.

## Shared invariants

1. Minimal diff. Do not rewrite the idea parser.
2. Never auto-commit. Atomic write `paper_draft.tex`.
3. Esc on the picker **cancels** the insert (do not write).
4. Same Sci-Action as C2: `EditDraft`.
5. Clippy clean.

## Requirements

1. After a non-empty note is committed in the capture modal, open a section picker list:
   - One row per draft `\section` title from current `paper_sections` / split
   - Final row: “End of draft” (`section_id = None`)
2. Choosing a section sets `IdeaBlock.section_id` to that title (existing convention) and writes via `update_or_insert_idea_block` + `write_atomic_str`.
3. Empty note still no-ops (C2).
4. Help overlay for `ReadingSourceMd` / the new picker mode lists the flow.
5. If D1 landed, register `CaptureNote` (already may exist) — picker is part of that command.
6. Unit tests:
   1. Picker + section “Introduction” produces a block with `section_id == Some("Introduction")`.
   2. Esc on picker does not change `paper_draft.tex`.
   3. “End of draft” still inserts a valid `from:` block (C2 contract).

## Out of scope

- Cite-into-section (R4)
- Undo journal (T1) — if T1 already merged, snapshot; if not, skip
- New idea parser syntax

## Verify

```bash
cargo test -p sil-tui -p sil-latex
cargo clippy -p sil-tui -p sil-latex --all-targets -- -D warnings
```

## Deliverable

Picker mode, how section titles are sourced, residual “structure.yaml ids not listed unless they match a `\section`”.
