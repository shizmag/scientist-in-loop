# PR-C2 — Reader note (`n`)

Copy the block below into an agent session (worktree-isolated if parallel with C1).

---

## Role

You are the **reader-note engineer** for scientist-in-loop. Ship ONLY PR-C2.

## Goal

From the markdown reader, `n` opens a one-line modal. On confirm, insert a `# -- X -- #` block into `paper_draft.tex` that points at the current source (`from: <filename>`). This is the human “this claim matters for my paper” verb. No new data model.

## Repo context

- Workspace: scientist-in-loop
- Parent plan: `docs/plan-08-13/pr-plan.md` §5.4, KD-10, KD-11, KD-12, KD-14, KD-15
- Reader handler: `handle_reading_source_md_mode` in `crates/sil-tui/src/app/handlers/mod.rs`
- Modal pattern: `InputMode::ModalAddSourceLink` / `ModalRenameSource` + `crates/sil-tui/src/ui/modals.rs`
- Insert: `sil_latex::update_or_insert_idea_block`
- Types: `sil_core::IdeaBlock`
- Write: `sil_core::write_atomic_str` on `paper_draft.tex`
- After write: reload draft (`App::reload_paper_draft`) so tab 4 and dashboard ideas stay true
- Help: `HelpMode::ReadingSourceMd`

## Shared invariants

1. Minimal diff. Reuse idea parser + atomic write. Do not add a SQLite reading-state column.
2. Never auto-commit. Sci-Action if you surface a proposal: `EditDraft` only. No new variant.
3. Same interface family: one-field modal, Enter confirm, Esc cancel.
4. Empty / whitespace note is a no-op (no file write).
5. Prefer unit tests; clippy clean on touched crates.

## Requirements

1. Add `InputMode::ModalCaptureNote` (and matching `HelpMode` if the help overlay is mode-aware).
2. `n` in `ReadingSourceMd` opens the modal. Prefill empty. Title like “Capture note for draft”.
3. On Enter (non-empty):
   - Build `IdeaBlock`:
     - `id`: `from-{source_id}-{short_hash(note)}` (stable, unique per note text)
     - `content`: first line `from: {filename}`, then the user note
     - `section_id`: `None` (append at end of draft — do **not** build a section picker)
     - `status`: `open`, `priority`: `medium`, `author_type`: `human`
     - `tags`: `["from-source"]`
   - `update_or_insert_idea_block` + `write_atomic_str`
   - Reload draft + ideas; status “Parked note from {filename}”
   - Return to `ReadingSourceMd` (do not dump the user to tab 4)
4. Esc / empty: close modal, stay in reader, no write.
5. Missing `paper_draft.tex`: status error, no panic.
6. Help overlay: `n` = “Park a note on paper_draft.tex (`from:` this source)”.
7. Tests:
   1. Formatter / insert: given a draft and a block, result contains `% from: attention.pdf` and the note text bounded by `# -- X -- #`.
   2. Empty note does not call write.
   3. Two different notes → two ids (no clobber).
   4. Keymap includes `n`.

Normative block:

```latex
% # -- X -- #
% from: attention.pdf
% Residual stream carries the unembedding (Smith 2024, §3)
% # -- X -- #
```

## Out of scope

- Reader `b` (C1)
- Pin to `structure.yaml` section
- Highlights / annotation layer
- Auto `\cite{}`
- Agent `sil_cite action=ground` changes
- New Sci-Action

## Verify

```bash
cargo test -p sil-tui -p sil-latex
cargo clippy -p sil-tui -p sil-latex --all-targets -- -D warnings
```

## Deliverable

New `InputMode`, insert helper, id scheme, help line, residual “notes append at end of draft (no section picker)”.
