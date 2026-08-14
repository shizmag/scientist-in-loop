# PR-T1 — Undo journal

Copy the block below into an agent session (worktree-isolated if parallel with R1/R2).

---

## Role

You are the **undo engineer** for scientist-in-loop. Ship ONLY PR-T1.

## Goal

TUI mutations that delete or insert into durable files can be undone. Keep a capped generation journal under `.sil/undo/` (gitignored). This is **not** `git checkout`.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-14/pr-plan.md` §5.7, KD-11
- Today: atomic writes exist; delete source has a confirm modal; no generation journal.
- Managed `.gitignore` is refreshed by `sil init --update` (`# >>> sil-managed`). Add `.sil/undo/`.

## Shared invariants

1. Minimal diff. Types in `sil-core` if possible (no Ratatui).
2. Never `git commit` / `git checkout` / `git restore`.
3. Covered ops only: delete source (DB + optional bib? — **files**: snapshot `references.bib` if you also delete a bib row; snapshot `paper_draft.tex` for note/cite; snapshot bib for bib delete). Source delete may be DB-only — snapshot whatever file/DB export is needed to restore **user-visible** state. Prefer file snapshots. If source delete is DB-only, restoring means re-inserting the source row + files already on disk (PDFs are not deleted? confirm current delete behavior — do not start deleting PDFs).
4. Cap **10** generations. Atomic journal writes.
5. Clippy clean.

## Requirements

1. `.sil/undo/` journal: incrementing ids, metadata + blobs (sidecar files are fine).
2. `Undo` command restores the last generation (bytes back). Optional `Redo` if cheap.
3. Hook **now** (in this PR) into existing TUI mutations you can see:
   - Delete bib entry
   - Delete source (restore DB row / list membership as current delete does inverse)
   - Reader note insert (if C2 path is easy to wrap)
   Cite-into-section (R4) will hook later — export a clear `undo::snapshot(paths, op, files)` API.
4. Palette: register `Undo` if D1 exists (`u` is already “use cache” on Settings — do **not** steal `u` globally). Prefer `Ctrl+Z` in Normal mode and palette “Undo”.
5. Tests:
   1. Delete bib → undo restores exact previous `references.bib` bytes.
   2. Note insert → undo restores previous `paper_draft.tex` bytes.
   3. Journal cap: 11th snapshot drops the oldest.
   4. `.gitignore` managed block mentions `.sil/undo/` (unit or e2e init --update if cheap).

## Out of scope

- General VCS, undo of Settings YAML unless you touch it (skip settings)
- Redo UI polish beyond a working stack
- `git` integration

## Verify

```bash
cargo test -p sil-core -p sil-tui
cargo clippy -p sil-core -p sil-tui --all-targets -- -D warnings
```

## Deliverable

Journal format, hooked mutations, keybinding (`Ctrl+Z` / palette), gitignore change.
