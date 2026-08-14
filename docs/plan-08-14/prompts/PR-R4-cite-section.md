# PR-R4 — Cite into section

Copy the block below into an agent session. **After R2 + T1.**

---

## Role

You are the **cite-insert engineer** for scientist-in-loop. Ship ONLY PR-R4.

## Goal

From the reader (or Sources), insert `\cite{key}` into a chosen draft section. Upsert the source into `references.bib` first if needed (`sil_app::upsert_bib`, `draft: true`). Snapshot undo. Never invent cite keys. Never commit.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.6, KD-9
- Today: reader `b` only upserts bib. No draft `\cite` insertion.
- Reuse R2 section picker widget.
- New helper: `sil_latex::insert_cite_in_section(tex, section_title, cite_key) -> String`.

## Shared invariants

1. Minimal diff. Reuse upsert + picker + undo API.
2. Never auto-commit. Sci-Action = `EditDraft` (no new variant).
3. Atomic write `paper_draft.tex`.
4. Clippy clean.

## Requirements

1. `insert_cite_in_section`:
   - Find `\section{title}` (or `\subsection` if that was picked).
   - Insert `\cite{key}` at end of that section body (before next `\section`/`\subsection` at same-or-higher level, or EOF).
   - If the key is already cited in that section, do not duplicate (status “already cited”).
   - If section missing, return error / no write.
2. Command `CiteIntoSection` (reader key `c` if free — **check collisions**; reader already has `b`/`n`/`j`/`k`. `c` is used as “add bib” in the refs viewer. In **ReadingSourceMd** `c` is likely free. If not, use palette only + a documented key).
3. Flow: upsert bib if needed → picker → insert → undo snapshot → reload draft.
4. Unit tests:
   1. Cite inserted only in the chosen section.
   2. Second cite of same key in same section is a no-op.
   3. Missing section does not write.
   4. File write is atomic (temp + rename pattern already in `write_atomic_str` — just call it).

## Out of scope

- Grounding modal (W3)
- New Sci-Action
- Auto-insert from MCP

## Verify

```bash
cargo test -p sil-latex -p sil-tui
cargo clippy -p sil-latex -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Helper behavior, keybinding, upsert-then-insert order, undo hook.
