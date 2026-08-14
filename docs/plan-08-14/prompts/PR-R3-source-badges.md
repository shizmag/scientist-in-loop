# PR-R3 — Derived source badges

Copy the block below into an agent session. **After R1.**

---

## Role

You are the **badge engineer** for scientist-in-loop. Ship ONLY PR-R3.

## Goal

Show derived badges on Sources rows: `parsed` / `unparsed`, `in bib`, `cited`. No new SQLite columns. Scientists can see library state without opening three tabs.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.5, KD-10
- Today: Sources list shows parse status (`[✓ Parsed]` / `[Unparsed]`). Bib membership and draft citation are derivable but not shown as a unified badge.
- ADR-015 KD-11: do **not** add triaged/reading columns.

## Shared invariants

1. Minimal diff. Render-time derivation only.
2. Never auto-commit.
3. No new DB schema.
4. Clippy clean.

## Requirements

1. Pure helper, e.g. `fn source_badges(source, bib_keys_or_entries, draft_tex) -> SourceBadges`.
2. Rules:
   - `parsed`: existing source flag
   - `in_bib`: cite-key / DOI / title match against loaded `references.bib` entries (be conservative: DOI or cite key first; title similarity optional — if you skip title, document it)
   - `cited`: `\cite{key}` / `\citep{key}` / `\citet{key}` contains that key in `paper_draft.tex`
3. Sources row suffix example: `[parsed · in bib · cited]` / `[unparsed]`.
4. Unit tests (no network):
   1. Unparsed + not in bib + not cited.
   2. Parsed + in bib by DOI + not cited.
   3. Parsed + in bib + `\cite{thatkey}` in draft → cited.
   4. Multi-key `\cite{a,b}` counts as cited for `a` and `b`.

## Out of scope

- Stored workflow states
- Split view
- Changing fetch/parse

## Verify

```bash
cargo test -p sil-tui
cargo clippy -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Helper rules, how rows render, residual “title-only match not implemented” if true.
