# ADR-017: Reader Note Section Picker Flow

## Status
Accepted (Wave 08-14 / PR-R2 / KD-8)

## Context
When reading a source paper in Markdown view (`ReadingSourceMd`), the reader note verb (`n`) allows the scientist to capture an idea block (`# -- X -- #`) linked to the source (`from: <filename>`). Previously, reader notes were placed either at the end of the manuscript or without an interactive section selection step. While `sil_latex::update_or_insert_idea_block` already natively supports `section_id` targeting, the user interface lacked a modal selection step to route the captured note into a specific `\section` of `paper_draft.tex`.

## Decisions

1. **Two-Step Modal Flow (`ModalCaptureNote` -> `NoteSectionPicker`)**:
   - Submitting note text with `Enter` in `InputMode::ModalCaptureNote` transitions to `InputMode::NoteSectionPicker` with the captured text stored in `App.pending_note_text`.
   - If the note buffer is empty on `Enter`, the action is a no-op that cleanly restores `InputMode::ReadingSourceMd` without creating an undo snapshot or altering `paper_draft.tex`.
2. **Draft Section Enumeration**:
   - Section candidates are extracted dynamically from `App.paper_sections` (filtered for heading kinds, excluding preamble / full document fallbacks).
   - An additional final row `None` representing `[End of draft]` is appended so the scientist can explicitly opt to park the note at the end of the draft (`section_id = None`).
3. **Section Targeting and Block Insertion**:
   - Selecting a section (`Some(section_title)` or `None`) invokes `save_reader_note_to_section`.
   - An undo snapshot is recorded via `sil_core::undo::snapshot` prior to mutation.
   - `sil_latex::update_or_insert_idea_block` is called with the target `section_id`, placing the block directly under the heading or at the end of the manuscript.
   - The file is written atomically via `sil_core::write_atomic_str`, draft sections are regenerated on disk, SQLite idea entries are updated, and the in-memory draft is reloaded.
4. **Cancellation Safety**:
   - Pressing `Esc` in `NoteSectionPicker` cancels note capture, clears the pending note buffer, leaves `paper_draft.tex` untouched, and restores `InputMode::ReadingSourceMd`.
5. **Help Overlay & Hints**:
   - Added `HelpMode::NoteSectionPicker` with keymap hints (`j`/`k`/Up/Down to navigate, `Enter` to confirm, `Esc` to cancel, `?`/`F1` for help overlay).
   - Added mode-aware status footer hints for `NoteSectionPicker`.

## Residuals & Out of Scope
- Cite-into-section (PR-R4) remains a separate workflow.
- Additional `structure.yaml` section IDs not present as `\section{...}` in LaTeX are not listed in the picker.
